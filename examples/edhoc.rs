use oscore::edhoc::{Msg1Receiver, Msg1Sender};

fn main() {
    // TODO: An EDHOC error message should be sent to the other party whenever
    // an operation fails and the protocol is abandoned.

    let v_public = [
        0x88, 0x3D, 0x9F, 0x20, 0xAF, 0x73, 0xF7, 0x8E, 0xD2, 0x94, 0x78,
        0xE4, 0x16, 0x51, 0x4B, 0x88, 0x57, 0x19, 0x64, 0x3B, 0x63, 0xC5,
        0x81, 0xFD, 0x8B, 0x57, 0xDD, 0x3A, 0xC8, 0x01, 0x1A, 0xC6,
    ];
    let u_public = [
        0xB3, 0x94, 0x7F, 0x71, 0xA5, 0xCC, 0xA4, 0xF1, 0xD2, 0xA3, 0x42,
        0xAE, 0x62, 0x24, 0x17, 0x5E, 0x83, 0x77, 0x49, 0x34, 0x7E, 0x54,
        0x21, 0x8C, 0x35, 0xED, 0x0C, 0xC8, 0x0A, 0x26, 0x69, 0x79,
    ];

    // Party U ----------------------------------------------------------------
    // "Generate" an ECDH key pair (this is static, but MUST be ephemeral)
    // The ECDH private key used by U
    let u_priv = [
        144, 115, 162, 206, 225, 72, 94, 30, 253, 17, 9, 171, 183, 84, 94, 17,
        170, 82, 95, 72, 77, 44, 124, 143, 102, 139, 156, 120, 63, 2, 27, 70,
    ];
    // Choose a connection identifier
    let u_c_u = b"Party U";

    let msg1_sender = Msg1Sender::new(u_c_u, u_priv);
    let (mut msg1_bytes, msg2_receiver) = msg1_sender.generate_message_1();

    // Party V ----------------------------------------------------------------
    // "Generate" an ECDH key pair (this is static, but MUST be ephemeral)
    // The ECDH private key used by V
    let v_priv = [
        16, 165, 169, 23, 227, 139, 247, 13, 53, 60, 173, 235, 46, 22, 199,
        69, 54, 240, 59, 183, 80, 23, 70, 121, 195, 57, 176, 97, 255, 171,
        154, 93,
    ];
    // Choose a connection identifier
    let v_c_v = b"Party V";

    let msg1_receiver = Msg1Receiver::new(v_c_v, v_priv);
    let msg2_sender = msg1_receiver.handle_message_1(&mut msg1_bytes);
    let (mut msg2_bytes, msg3_receiver) = msg2_sender.generate_message_2();

    // Party U ----------------------------------------------------------------
    let msg3_sender =
        msg2_receiver.handle_message_2(&mut msg2_bytes, v_public);
    let (mut msg3_bytes, u_master_secret, u_master_salt) =
        msg3_sender.generate_message_3();

    // Party V ----------------------------------------------------------------
    let (v_master_secret, v_master_salt) =
        msg3_receiver.handle_message_3(&mut msg3_bytes, u_public);

    // Party U ----------------------------------------------------------------
    // It's possible that Party V failed verification of message_3, in which
    // case it sends an EDHOC error message.
    // Technically, Party U would have to be ready to receive this message and
    // invalidate any protocol state.

    // Verification -----------------------------------------------------------
    assert_eq!(u_master_secret, v_master_secret);
    assert_eq!(u_master_salt, v_master_salt);

    println!(
        "OSCORE Context established.\n\
         Master Secret:\n{}\n\
         Master Salt:\n{}",
        hexstring(&u_master_secret),
        hexstring(&u_master_salt)
    );
}

fn hexstring(slice: &[u8]) -> String {
    String::from("0x")
        + &slice
            .iter()
            .map(|n| format!("{:02X}", n))
            .collect::<Vec<String>>()
            .join(", 0x")
}
