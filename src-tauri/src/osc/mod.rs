pub mod commands;

use rosc::{encoder, OscMessage, OscPacket, OscType};
use std::net::UdpSocket;

const VRC_OSC_ADDR: &str = "127.0.0.1:9000";

pub fn send_avatar_change(avatar_id: &str) -> anyhow::Result<()> {
    let packet = OscPacket::Message(OscMessage {
        addr: "/avatar/change".to_string(),
        args: vec![OscType::String(avatar_id.to_string())],
    });
    let encoded = encoder::encode(&packet)?;
    let socket = UdpSocket::bind("0.0.0.0:0")?;
    socket.send_to(&encoded, VRC_OSC_ADDR)?;
    Ok(())
}

pub fn send_parameter(address: &str, value: OscType) -> anyhow::Result<()> {
    let packet = OscPacket::Message(OscMessage {
        addr: address.to_string(),
        args: vec![value],
    });
    let encoded = encoder::encode(&packet)?;
    let socket = UdpSocket::bind("0.0.0.0:0")?;
    socket.send_to(&encoded, VRC_OSC_ADDR)?;
    Ok(())
}
