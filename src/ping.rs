use crate::{
    errors::Error,
    packet::{EchoReply, EchoRequest, IcmpV4, IcmpV6, IpV4Packet, ICMP_HEADER_SIZE},
};
use socket2::{Domain, Protocol, Socket, Type};
use std::{
    cell::Cell,
    mem::MaybeUninit,
    net::{IpAddr, SocketAddr},
    time::Duration,
};

pub const TOKEN_SIZE: usize = 24;
const ECHO_REQUEST_BUFFER_SIZE: usize = ICMP_HEADER_SIZE + TOKEN_SIZE;
pub type Token = [u8; TOKEN_SIZE];

thread_local!(
    static IDENT: Cell<u16> = Cell::new(0);
);

pub fn ping(
    addr: IpAddr,
    timeout: Option<Duration>,
    ttl: Option<u32>,
    ident: Option<u16>,
    seq_cnt: Option<u16>,
    payload: Option<&Token>,
) -> Result<(), Error> {
    let timeout = match timeout {
        Some(timeout) => Some(timeout),
        None => Some(Duration::from_secs(4)),
    };

    let dest = SocketAddr::new(addr, 0);
    let mut buffer = [0; ECHO_REQUEST_BUFFER_SIZE];

    let request = EchoRequest {
        ident: ident.unwrap_or_else(|| {
            IDENT.with(|cell| {
                let ident = cell.get();
                cell.set(ident.wrapping_add(1));
                ident
            })
        }),
        seq_cnt: seq_cnt.unwrap_or(1),
        payload: payload.unwrap_or_else(|| &[0; TOKEN_SIZE]),
    };

    let socket = if dest.is_ipv4() {
        if request.encode::<IcmpV4>(&mut buffer[..]).is_err() {
            return Err(Error::InternalError);
        }
        Socket::new(Domain::IPV4, Type::RAW, Some(Protocol::ICMPV4))?
    } else {
        if request.encode::<IcmpV6>(&mut buffer[..]).is_err() {
            return Err(Error::InternalError);
        }
        Socket::new(Domain::IPV6, Type::RAW, Some(Protocol::ICMPV6))?
    };

    socket.set_ttl(ttl.unwrap_or(64))?;

    socket.set_write_timeout(timeout)?;

    socket.send_to(&buffer, &dest.into())?;

    socket.set_read_timeout(timeout)?;

    fn assume_init(buf: &[MaybeUninit<u8>]) -> &[u8] {
        unsafe { &*(buf as *const [MaybeUninit<u8>] as *const [u8]) }
    }

    let mut buffer: [MaybeUninit<u8>; 2048] =
        unsafe { [MaybeUninit::zeroed().assume_init(); 2048] };
    socket.recv_from(&mut buffer)?;

    let _reply = if dest.is_ipv4() {
        let ipv4_packet = match IpV4Packet::decode(assume_init(&buffer)) {
            Ok(packet) => packet,
            Err(_) => return Err(Error::InternalError),
        };
        match EchoReply::decode::<IcmpV4>(ipv4_packet.data) {
            Ok(reply) => reply,
            Err(_) => return Err(Error::InternalError),
        }
    } else {
        match EchoReply::decode::<IcmpV6>(assume_init(&buffer)) {
            Ok(reply) => reply,
            Err(_) => return Err(Error::InternalError),
        }
    };

    Ok(())
}
