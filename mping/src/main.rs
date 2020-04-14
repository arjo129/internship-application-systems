use std::env;
use std::process;
use std::net::{ Ipv4Addr, IpAddr, SocketAddrV4};
use std::time::{Duration, SystemTime};
use socket2::{Socket, Domain, Protocol, SockAddr, Type}; 

const ECHO_REQUEST_TYPE: u8 = 8;
const ECHO_REQUEST_CODE: u8 = 0;
const ECHO_REPLY_TYPE: u8 = 129;
const ECHO_REPLY_CODE: u8 = 0;

const PROGRAM_ID: u16 = 0xFA9E;

pub struct EchoRequestPacket {
    pub identity: u16,
    pub seq_cnt: u16
}

impl EchoRequestPacket {
    
    fn new(identity: u16, seq_cnt: u16) -> Self {
        let p:EchoRequestPacket = EchoRequestPacket {identity, seq_cnt};
        p
    }

    fn serialize(&self, buff: &mut[u8]) {
        
        assert!(buff.len() >= 24);

        buff[0] = ECHO_REQUEST_TYPE;
        buff[1] = ECHO_REQUEST_CODE;
        buff[4] = (self.identity >> 8) as u8;
        buff[5] = self.identity as u8;
        buff[6] = (self.seq_cnt >> 8) as u8;
        buff[7] = self.seq_cnt as u8;

        // Put the time stamp in the echo packet
        let now = SystemTime::now().duration_since(SystemTime::UNIX_EPOCH).expect("Cannot get system time???");
        let mut time_elapsed = now.as_millis();
        for i in 0..16 {
            buff[8+i] = (time_elapsed & 0xFF) as u8;
            time_elapsed >>= 8; 
        }
        calc_checksum(buff);
    }
}


fn calc_checksum(buffer: &mut [u8]) {
    
    let mut checksum = 0u32;

    for word in buffer.chunks(2) {
        let mut part = u16::from(word[0]) << 8;
        if word.len() > 1 {
            part += u16::from(word[1]);
        }
        checksum = checksum.wrapping_add(u32::from(part));
    }

    while (checksum >> 16) > 0 {
        checksum = (checksum & 0xffff) + (checksum >> 16);
    }

    let sum = !checksum as u16;

    buffer[2] = (sum >> 8) as u8;
    buffer[3] = (sum & 0xff) as u8;
}

fn handle_response(buffer: &[u8], length: usize) {

    if length < 32 {
        println!("Error decoding packet. Incomplete header");
    }

    let now = SystemTime::now().duration_since(SystemTime::UNIX_EPOCH).expect("Cannot get system time???"); 

    let mut time_elapsed  = 0u128;
    for i in 0..16 {
        time_elapsed |= (buffer[28+i] as u128) << (8*i);  
    }

    println!("{}", now.as_millis() - time_elapsed);
}

fn main() {
    let args: Vec<String> = env::args().collect();
    
    /*if args.len() != 2 {
        println!("Usage: mping <hostname/ip>");
        process::exit(-1);
    }*/

    
    let address = SockAddr::from(SocketAddrV4::new(Ipv4Addr::new(8,8,8,8), 7));
    let socket = Socket::new(Domain::ipv4(), Type::raw(), Some(Protocol::icmpv4())).expect("Failed to open socket.\n This could be a permissions problem as the program writes to a raw socket.\n To resolve set socket capability");
    let mut data_buf = [0u8; 24];
    let packet = EchoRequestPacket::new(PROGRAM_ID,1);
    packet.serialize(&mut data_buf);
    println!("{:?}", data_buf);
    
    socket.send_to(&mut data_buf, &address).expect("send failed");
    let mut data_buf = [0u8; 128];
    let length = socket.recv(&mut data_buf).expect("receive failed");
    handle_response(&data_buf, length);
    println!("{}", length);
    //println!("{:?}", data_buf);
}
