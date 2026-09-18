//! NEMES minimal P2P node (VPS-less): TCP + Noise + Yamux + Gossipsub + mDNS + Identify

/// Gorev duyuru topigi: komuta dagitimi burada ilan eder, miner uyanir.
/// Gercek gorev yukü HTTP ile alinir (hassas veri gossip'e konmaz).
pub const GOREV_TOPIC: &str = "nemes/gorev";

use futures::StreamExt;
use libp2p::Transport as _;
use libp2p::{
    gossipsub, identify, kad, mdns, noise,
    swarm::{NetworkBehaviour, SwarmEvent},
    tcp, yamux, Multiaddr, PeerId as Libp2pPeerId, Swarm,
};
use std::{collections::HashMap, sync::Arc, time::Duration};
use tokio::sync::RwLock;
use tracing::{debug, info};

#[derive(Debug, Clone)]
pub struct P2PConfig {
    pub port: u16,
    pub enable_mdns: bool,
    /// WAN tohum adresleri (`/ip4/.../tcp/.../p2p/...` virgullu de olabilir).
    /// Bos = yalnizca LAN (mDNS) kesfi (eski davranis).
    pub bootstrap: Vec<String>,
    /// Sabit dugum kimligi tohumu (32B). Verilirse PeerId kararli olur
    /// (tohum listeleri curumez); verilmezse her acilista gecici kimlik
    /// uretilir (eski davranis).
    pub key_seed: Option<[u8; 32]>,
}

impl Default for P2PConfig {
    fn default() -> Self {
        Self { port: 4001, enable_mdns: true, bootstrap: Vec::new(), key_seed: None }
    }
}

#[derive(Debug, Clone)]
pub enum NetworkEvent {
    PeerConnected(Libp2pPeerId),
    PeerDisconnected(Libp2pPeerId),
    MessageReceived { from: Libp2pPeerId, topic: String, data: Vec<u8> },
}

#[derive(NetworkBehaviour)]
#[behaviour(out_event = "NetEvent")]
pub struct NetBehaviour {
    pub gossipsub: gossipsub::Behaviour,
    pub mdns: mdns::tokio::Behaviour,
    pub identify: identify::Behaviour,
    pub kademlia: kad::Behaviour<kad::store::MemoryStore>,
}

#[derive(Debug)]
pub enum NetEvent {
    Gossipsub(gossipsub::Event),
    Mdns(mdns::Event),
    Identify(identify::Event),
    Kademlia(kad::Event),
}

impl From<gossipsub::Event> for NetEvent {
    fn from(e: gossipsub::Event) -> Self { NetEvent::Gossipsub(e) }
}
impl From<mdns::Event> for NetEvent {
    fn from(e: mdns::Event) -> Self { NetEvent::Mdns(e) }
}
impl From<identify::Event> for NetEvent {
    fn from(e: identify::Event) -> Self { NetEvent::Identify(e) }
}
impl From<kad::Event> for NetEvent {
    fn from(e: kad::Event) -> Self { NetEvent::Kademlia(e) }
}

pub struct P2PNode {
    swarm: Swarm<NetBehaviour>,
    event_tx: tokio::sync::mpsc::UnboundedSender<NetworkEvent>,
    event_rx: Option<tokio::sync::mpsc::UnboundedReceiver<NetworkEvent>>,
    peers: Arc<RwLock<HashMap<Libp2pPeerId, Vec<Multiaddr>>>>,
    mdns_acik: bool,
}

impl P2PNode {
    pub async fn new(cfg: P2PConfig) -> anyhow::Result<Self> {
        // Sabit kimlik (B18 WAN): tohum verildiyse PeerId kararli olur.
        let keypair = match cfg.key_seed {
            Some(seed) => {
                let secret = libp2p::identity::ed25519::SecretKey::try_from_bytes(seed)
                    .map_err(|e| anyhow::anyhow!("p2p tohum: {}", e))?;
                libp2p::identity::Keypair::from(
                    libp2p::identity::ed25519::Keypair::from(secret),
                )
            }
            None => libp2p::identity::Keypair::generate_ed25519(),
        };
        let peer_id = Libp2pPeerId::from(keypair.public());
        info!("local peer id: {}", peer_id);

        let transport = tcp::tokio::Transport::new(tcp::Config::default().nodelay(true))
            .upgrade(libp2p::core::upgrade::Version::V1Lazy)
            .authenticate(noise::Config::new(&keypair)?)
            .multiplex(yamux::Config::default())
            .timeout(Duration::from_secs(20))
            .boxed();

        let gossipsub_config = gossipsub::ConfigBuilder::default()
            .heartbeat_interval(Duration::from_secs(10))
            .validation_mode(gossipsub::ValidationMode::Strict)
            .build()
            .map_err(|e| anyhow::anyhow!(e))?;
        let gossipsub = gossipsub::Behaviour::new(
            gossipsub::MessageAuthenticity::Signed(keypair.clone()),
            gossipsub_config,
        )
        .map_err(|e| anyhow::anyhow!("gossipsub: {}", e))?;
        let mdns = mdns::tokio::Behaviour::new(mdns::Config::default(), peer_id)?;
        let identify = identify::Behaviour::new(identify::Config::new(
            "/nemes/1.0.0".to_string(),
            keypair.public(),
        ));
        // Kademlia DHT (B18 WAN kesif): mDNS'in goremedigi esler icin.
        let mut kad_cfg = kad::Config::default();
        kad_cfg.set_protocol_names(vec![libp2p::StreamProtocol::new("/nemes/kad/1.0.0")]);
        let kademlia = kad::Behaviour::with_config(
            peer_id,
            kad::store::MemoryStore::new(peer_id),
            kad_cfg,
        );
        let behaviour = NetBehaviour { gossipsub, mdns, identify, kademlia };
        let mut swarm = Swarm::new(
            transport,
            behaviour,
            peer_id,
            libp2p::swarm::Config::with_tokio_executor(),
        );
        swarm.listen_on(format!("/ip4/0.0.0.0/tcp/{}", cfg.port).parse()?)?;
        for t in ["nemes/komut", "nemes/shard", "nemes/chat", GOREV_TOPIC, nemes_core::mesh_audit::DENETIM_TOPIC] {
            swarm.behaviour_mut().gossipsub.subscribe(&gossipsub::IdentTopic::new(t))?;
        }
        // Tohumlar: adresi kad'e ekle + ara, sonra bootstrap sorgusu.
        // Bozuk adres atlanir (dugum ayakta kalir).
        let mut tohum_sayisi = 0;
        for ham in cfg.bootstrap.iter().flat_map(|s| s.split(',')).map(str::trim).filter(|s| !s.is_empty()) {
            match ham.parse::<Multiaddr>() {
                Ok(addr) => match addr.iter().find_map(|p| match p {
                    libp2p::multiaddr::Protocol::P2p(id) => Some(id),
                    _ => None,
                }) {
                    Some(id) => {
                        swarm.behaviour_mut().kademlia.add_address(&id, addr.clone());
                        if swarm.dial(addr).is_ok() {
                            tohum_sayisi += 1;
                        }
                    }
                    None => debug!("tohumda peer id yok, atlandi"),
                },
                Err(_) => debug!("bozuk tohum adresi, atlandi"),
            }
        }
        if tohum_sayisi > 0 {
            match swarm.behaviour_mut().kademlia.bootstrap() {
                Ok(_) => info!("kad bootstrap basladi ({} tohum)", tohum_sayisi),
                Err(e) => debug!("kad bootstrap ertelendi: {}", e),
            }
        }
        let (event_tx, event_rx) = tokio::sync::mpsc::unbounded_channel();
        let mdns_acik = cfg.enable_mdns;
        Ok(Self { swarm, event_tx, event_rx: Some(event_rx), peers: Arc::new(RwLock::new(HashMap::new())), mdns_acik })
    }

    /// Yerel es kimligi (tohum listeleri icin).
    pub fn peer_id(&self) -> Libp2pPeerId {
        *self.swarm.local_peer_id()
    }

    /// Dinlenen adresler (tohum uretimi icin).
    pub fn dinleyiciler(&self) -> Vec<Multiaddr> {
        self.swarm.listeners().cloned().collect()
    }

    /// Kademlia bootstrap sorgusunu elle tetikler (es biliniyorsa).
    pub fn bootstrap(&mut self) -> anyhow::Result<()> {
        self.swarm
            .behaviour_mut()
            .kademlia
            .bootstrap()
            .map(|_| ())
            .map_err(|e| anyhow::anyhow!("kad bootstrap: {}", e))
    }

    pub fn take_event_receiver(&mut self) -> Option<tokio::sync::mpsc::UnboundedReceiver<NetworkEvent>> {
        self.event_rx.take()
    }

    pub fn publish(&mut self, topic: &str, data: Vec<u8>) -> anyhow::Result<()> {
        self.swarm.behaviour_mut().gossipsub
            .publish(gossipsub::IdentTopic::new(topic), data)
            .map(|_| ())
            .map_err(|e| anyhow::anyhow!(e))
    }

    pub fn dial(&mut self, addr: Multiaddr) -> anyhow::Result<()> {
        self.swarm.dial(addr)?;
        Ok(())
    }

    pub fn connected_peers_count(&self) -> usize {
        self.swarm.connected_peers().count()
    }

    /// Swarm'i belirli sure calistir (saniye). Baglanti kurmak icin kullanilabilir.
    pub async fn run_for(&mut self, secs: u64) -> anyhow::Result<()> {
        let deadline = tokio::time::Instant::now() + Duration::from_secs(secs);
        while tokio::time::Instant::now() < deadline {
            match tokio::time::timeout(Duration::from_millis(500), self.swarm.select_next_some()).await {
                Ok(event) => {
                    match event {
                        SwarmEvent::Behaviour(NetEvent::Gossipsub(gossipsub::Event::Message { propagation_source, message, .. })) => {
                            let _ = self.event_tx.send(NetworkEvent::MessageReceived {
                                from: propagation_source,
                                topic: message.topic.to_string(),
                                data: message.data,
                            });
                        }
                        SwarmEvent::Behaviour(NetEvent::Mdns(mdns::Event::Discovered(list))) => {
                            // enable_mdns=false ise kesif islenmez (bayrak artik gercek).
                            if self.mdns_acik {
                                for (peer, addr) in list {
                                    debug!("mdns discovered {} {}", peer, addr);
                                    self.swarm.dial(addr.clone())?;
                                }
                            }
                        }
                        SwarmEvent::Behaviour(NetEvent::Kademlia(ev)) => {
                            debug!("kad: {:?}", ev);
                        }
                        SwarmEvent::ConnectionEstablished { peer_id, .. } => {
                            info!("connected {}", peer_id);
                            self.peers.write().await.insert(peer_id, vec![]);
                            let _ = self.event_tx.send(NetworkEvent::PeerConnected(peer_id));
                        }
                        SwarmEvent::ConnectionClosed { peer_id, .. } => {
                            self.peers.write().await.remove(&peer_id);
                            let _ = self.event_tx.send(NetworkEvent::PeerDisconnected(peer_id));
                        }
                        _ => {}
                    }
                }
                Err(_) => {
                    // timeout - continue loop
                }
            }
        }
        Ok(())
    }

    pub async fn run(mut self) -> anyhow::Result<()> {
        loop {
            match self.swarm.select_next_some().await {
                SwarmEvent::Behaviour(NetEvent::Gossipsub(gossipsub::Event::Message { propagation_source, message, .. })) => {
                    let _ = self.event_tx.send(NetworkEvent::MessageReceived {
                        from: propagation_source,
                        topic: message.topic.to_string(),
                        data: message.data,
                    });
                }
                SwarmEvent::Behaviour(NetEvent::Mdns(mdns::Event::Discovered(list))) => {
                    if self.mdns_acik {
                        for (peer, addr) in list {
                            debug!("mdns discovered {} {}", peer, addr);
                            self.swarm.dial(addr.clone())?;
                        }
                    }
                }
                SwarmEvent::Behaviour(NetEvent::Kademlia(ev)) => {
                    debug!("kad: {:?}", ev);
                }
                SwarmEvent::ConnectionEstablished { peer_id, .. } => {
                    info!("connected {}", peer_id);
                    self.peers.write().await.insert(peer_id, vec![]);
                    let _ = self.event_tx.send(NetworkEvent::PeerConnected(peer_id));
                }
                SwarmEvent::ConnectionClosed { peer_id, .. } => {
                    self.peers.write().await.remove(&peer_id);
                    let _ = self.event_tx.send(NetworkEvent::PeerDisconnected(peer_id));
                }
                _ => {}
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// B18: mDNS kapaliyken iki dugum tohum adresiyle bulusur.
    #[tokio::test]
    async fn test_bootstrap_baglanir() {
        let mut tohum = P2PNode::new(P2PConfig {
            port: 0,
            enable_mdns: false,
            bootstrap: vec![],
            key_seed: Some([7u8; 32]),
        })
        .await
        .expect("tohum dugum acilmali");
        // Dinleyici adresi ilk poll'den sonra belirir.
        let _ = tohum.run_for(1).await;
        // Dinlenen gercek adres + es kimlikten tohum dizgisi kur.
        let pid = tohum.peer_id();
        let port: u16 = tohum
            .dinleyiciler()
            .iter()
            .filter_map(|a| {
                a.iter().find_map(|p| match p {
                    libp2p::multiaddr::Protocol::Tcp(p) => Some(p),
                    _ => None,
                })
            })
            .next()
            .expect("dinleyici portu okunmali");
        let tohum_adres = format!("/ip4/127.0.0.1/tcp/{}/p2p/{}", port, pid);
        let mut gezgin = P2PNode::new(P2PConfig {
            port: 0,
            enable_mdns: false,
            bootstrap: vec![tohum_adres],
            key_seed: Some([9u8; 32]),
        })
        .await
        .expect("gezgin dugum acilmali");
        let mut gezgin_rx = gezgin.take_event_receiver().expect("olay kanali");
        let deadline = tokio::time::Instant::now() + Duration::from_secs(15);
        // Her iki dugumu de pompla; baglanti gezgine PeerConnected olarak duser.
        let baglandi = loop {
            if tokio::time::Instant::now() > deadline {
                break false;
            }
            let _ = tohum.run_for(1).await;
            let _ = gezgin.run_for(1).await;
            let mut buldu = false;
            while let Ok(ev) = gezgin_rx.try_recv() {
                if matches!(ev, NetworkEvent::PeerConnected(_)) {
                    buldu = true;
                    break;
                }
            }
            if buldu {
                break true;
            }
        };
        assert!(baglandi, "tohumla baglanti 15sn'de kurulmali");
    }

    /// Bozuk tohum adresi dugumu oldurmez.
    #[tokio::test]
    async fn test_bozuk_tohum_atlanir() {
        let n = P2PNode::new(P2PConfig {
            port: 0,
            enable_mdns: false,
            bootstrap: vec!["bozuk-adres".to_string(), "/ip4/127.0.0.1/tcp/9/p2p/12D3KooW9bogus".to_string()],
            key_seed: None,
        })
        .await;
        assert!(n.is_ok(), "bozuk tohum yoksayilmali");
    }
}
