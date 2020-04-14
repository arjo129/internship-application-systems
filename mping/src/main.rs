use std::env;
use std::process;
use std::io::ErrorKind;
use std::net::{ ToSocketAddrs};
use std::time::{Duration, SystemTime};
use socket2::{Socket, Domain, Protocol, SockAddr, Type}; 

const ECHO_REQUEST_TYPE: u8 = 8;
const ECHO_REQUEST_CODE: u8 = 0;
//const ECHO_REPLY_TYPE: u8 = 129;
//const ECHO_REPLY_CODE: u8 = 0;
const ECHO_TTL_EXCEEDED: u8 = 11;

const TX_INTERVAL: u64 = 200;
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
        
        for i in 0..buff.len() {
            buff[i] = 0;
        }

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
    
    let mut checksum = 0i32;

    for word in buffer.chunks(2) {
        let mut part = u16::from(word[0]) <<8;
        if word.len() > 1 {
            part += u16::from(word[1]);
        }
        checksum = checksum.wrapping_add(i32::from(part));
    }

    while (checksum >> 16) > 0 {
        checksum = (checksum & 0xffff) + (checksum >> 16);
    }

    let sum = !checksum as u16;

    buffer[2] = (sum >> 8) as u8;
    buffer[3] = (sum & 0xff) as u8;
}

#[derive(Debug, Copy, Clone)]
pub struct ResponseHandler {
    packets_on_wire: u16,
    maxcount: u16
}

impl ResponseHandler {
    fn handle_response(&mut self, buffer: &[u8], length: usize) {

        if length < 32 {
            println!("Error decoding packet. Incomplete header");
        }

        let now = SystemTime::now().duration_since(SystemTime::UNIX_EPOCH).expect("Cannot get system time???"); 

        let mut time_elapsed  = 0u128;

        if buffer[20] == ECHO_TTL_EXCEEDED {
            println!("TTL max hops exceeded");
            return;
        }

        for i in 0..16 {
            time_elapsed |= (buffer[28+i] as u128) << (8*i);  
        }

        let seq_num = (buffer[26] as u16) << 8 | (buffer[27] as u16);
        self.packets_on_wire-=1;
        let packet_loss = 100f64 * (self.packets_on_wire as f64 / self.maxcount as f64);
        println!("RTT: {}ms, ICMP Sequence: {}, Packet Loss: {}%", now.as_millis() - time_elapsed, seq_num, packet_loss);
    }

    fn insert_response(&mut self){
        self.packets_on_wire+=1;
        self.maxcount+=1;
    }
    
}

fn main() {
    let args: Vec<String> = env::args().collect();
    
    if args.len() < 2 {
        println!("Usage: mping <hostname/ip>");
        println!("Optional 3rd  argument to set ttl");
        process::exit(-1);
    }

    //Set the TTL
    let mut ttl = 64u32;
    if args.len() == 3 {
        ttl = args[2].parse::<u32>().expect("TTL argument must be an integer");
    }

    //Hackjob vause rust doesn't yet have nice get by hostnames api. It is assumed all ip addresses have a port.
    let mut raw_address = args[1].clone();
    raw_address.push_str(":0");
    let mut addresses = raw_address.to_socket_addrs().expect("Could not resolve hostname");
    let address_chosen = match addresses.next() {
        Some(addr) => addr,
        None => panic!("Could not resolve hostname")
    };

    let address = SockAddr::from(address_chosen);
    let socket = Socket::new(Domain::ipv4(), Type::raw(), Some(Protocol::icmpv4())).expect("Failed to open socket.\nThis could be a permissions problem as the program writes to a raw socket.\nTo resolve set socket capability");
    let mut data_buf = [0u8; 24];
    let mut counter = 0u16;
    let mut last_tx = SystemTime::UNIX_EPOCH;
    let mut handler = ResponseHandler{packets_on_wire: 0, maxcount: 0};
    socket.set_ttl(ttl).expect("invalide TTL set.");
    loop {
        socket.set_read_timeout(Some(Duration::from_millis(TX_INTERVAL))).expect("Failed to set time out on read");
        if SystemTime::now() > last_tx + Duration::from_millis(TX_INTERVAL) {  
            let packet = EchoRequestPacket::new(PROGRAM_ID, counter);
            counter+=1;
            handler.insert_response();
            packet.serialize(&mut data_buf);
            //println!("here");
            socket.send_to(&mut data_buf, &address).expect("send failed");
            last_tx = SystemTime::now();
        }
        let mut data_buf = [0u8; 128];
        let res = socket.recv(&mut data_buf);
        match res {
            Ok(length) => handler.handle_response(&mut data_buf, length),
            Err(e) => if e.kind() == ErrorKind::WouldBlock {
                //continue nothing wrong
                //println!("timeout");
            }
            else{
                panic!("Something went wrong {}", e);
            }
        }
    }   
}
