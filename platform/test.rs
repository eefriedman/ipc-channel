// Copyright 2015 The Servo Project Developers. See the COPYRIGHT
// file at the top-level directory of this distribution.
//
// Licensed under the Apache License, Version 2.0 <LICENSE-APACHE or
// http://www.apache.org/licenses/LICENSE-2.0> or the MIT license
// <LICENSE-MIT or http://opensource.org/licenses/MIT>, at your
// option. This file may not be copied, modified, or distributed
// except according to those terms.

use libc;
use platform::{OsIpcReceiver, OsIpcSender};
use std::iter;

#[test]
fn simple() {
    let rx = OsIpcReceiver::new().unwrap();
    let tx = rx.sender().unwrap();
    let data: &[u8] = b"1234567";
    tx.send(data, Vec::new()).unwrap();
    let (mut received_data, received_channels) = rx.recv().unwrap();
    received_data.truncate(7);
    assert_eq!((&received_data[..], received_channels), (data, Vec::new()));
}

#[test]
fn channel_transfer() {
    let (super_rx, sub_rx) = (OsIpcReceiver::new().unwrap(), OsIpcReceiver::new().unwrap());
    let (super_tx, sub_tx) = (super_rx.sender().unwrap(), sub_rx.sender().unwrap());
    let data: &[u8] = b"foo";
    super_tx.send(data, vec![sub_tx]).unwrap();
    let (_, mut received_channels) = super_rx.recv().unwrap();
    assert_eq!(received_channels.len(), 1);
    let sub_tx = received_channels.pop().unwrap();
    sub_tx.send(data, vec![]).unwrap();
    let (mut received_data, received_channels) = sub_rx.recv().unwrap();
    received_data.truncate(3);
    assert_eq!((&received_data[..], received_channels), (data, Vec::new()));
}

#[test]
fn multichannel_transfer() {
    let (super_rx, sub0_rx, sub1_rx) = (OsIpcReceiver::new().unwrap(),
                                        OsIpcReceiver::new().unwrap(),
                                        OsIpcReceiver::new().unwrap());
    let (super_tx, sub0_tx, sub1_tx) = (super_rx.sender().unwrap(),
                                        sub0_rx.sender().unwrap(),
                                        sub1_rx.sender().unwrap());
    let data: &[u8] = b"asdfasdf";
    super_tx.send(data, vec![sub0_tx, sub1_tx]).unwrap();
    let (_, mut received_channels) = super_rx.recv().unwrap();
    assert_eq!(received_channels.len(), 2);

    let sub0_tx = received_channels.remove(0);
    sub0_tx.send(data, vec![]).unwrap();
    let (mut received_data, received_subchannels) = sub0_rx.recv().unwrap();
    received_data.truncate(8);
    assert_eq!((&received_data[..], received_subchannels), (data, Vec::new()));

    let sub1_tx = received_channels.remove(0);
    sub1_tx.send(data, vec![]).unwrap();
    let (mut received_data, received_subchannels) = sub1_rx.recv().unwrap();
    received_data.truncate(8);
    assert_eq!((&received_data[..], received_subchannels), (data, Vec::new()));
}

#[test]
fn big_data() {
    let data: Vec<u8> = iter::repeat(0xba).take(65536).collect();
    let data: &[u8] = &data[..];
    let rx = OsIpcReceiver::new().unwrap();
    let tx = rx.sender().unwrap();
    tx.send(data, Vec::new()).unwrap();
    let (mut received_data, received_channels) = rx.recv().unwrap();
    received_data.truncate(65536);
    assert_eq!((&received_data[..], received_channels), (&data[..], Vec::new()));
}

#[test]
fn big_data_with_channel_transfer() {
    let data: Vec<u8> = iter::repeat(0xba).take(65536).collect();
    let data: &[u8] = &data[..];
    let (super_rx, sub_rx) = (OsIpcReceiver::new().unwrap(), OsIpcReceiver::new().unwrap());
    let (super_tx, sub_tx) = (super_rx.sender().unwrap(), sub_rx.sender().unwrap());
    super_tx.send(data, vec![sub_tx]).unwrap();
    let (_, mut received_channels) = super_rx.recv().unwrap();
    assert_eq!(received_channels.len(), 1);
    let sub_tx = received_channels.pop().unwrap();
    sub_tx.send(data, vec![]).unwrap();
    let (mut received_data, received_channels) = sub_rx.recv().unwrap();
    received_data.truncate(65536);
    assert_eq!((&received_data[..], received_channels), (data, Vec::new()));
}

#[test]
fn global_name_registration() {
    let rx = OsIpcReceiver::new().unwrap();
    let name = rx.register_global_name().unwrap();
    let tx = OsIpcSender::from_global_name(name).unwrap();

    let data: &[u8] = b"1234567";
    tx.send(data, Vec::new()).unwrap();
    let (mut received_data, received_channels) = rx.recv().unwrap();
    received_data.truncate(7);
    assert_eq!((&received_data[..], received_channels), (data, Vec::new()));
}

#[test]
fn cross_process() {
    let rx = OsIpcReceiver::new().unwrap();
    let name = rx.register_global_name().unwrap();
    let data: &[u8] = b"1234567";

    unsafe {
        if libc::fork() == 0 {
            let tx = OsIpcSender::from_global_name(name).unwrap();
            tx.send(data, Vec::new()).unwrap();
            libc::exit(0);
        }
    }

    let (mut received_data, received_channels) = rx.recv().unwrap();
    received_data.truncate(7);
    assert_eq!((&received_data[..], received_channels), (data, Vec::new()));
}

#[test]
fn cross_process_channel_transfer() {
    let super_rx = OsIpcReceiver::new().unwrap();
    let name = super_rx.register_global_name().unwrap();

    unsafe {
        if libc::fork() == 0 {
            let super_tx = OsIpcSender::from_global_name(name).unwrap();
            let sub_rx = OsIpcReceiver::new().unwrap();
            let sub_tx = sub_rx.sender().unwrap();
            let data: &[u8] = b"foo";
            super_tx.send(data, vec![sub_tx]).unwrap();
            sub_rx.recv().unwrap();
            let data: &[u8] = b"bar";
            super_tx.send(data, Vec::new()).unwrap();
            libc::exit(0);
        }
    }

    let (_, mut received_channels) = super_rx.recv().unwrap();
    assert_eq!(received_channels.len(), 1);
    let sub_tx = received_channels.pop().unwrap();
    let data: &[u8] = b"baz";
    sub_tx.send(data, Vec::new()).unwrap();

    let data: &[u8] = b"bar";
    let (mut received_data, received_channels) = super_rx.recv().unwrap();
    received_data.truncate(3);
    assert_eq!((&received_data[..], received_channels), (data, Vec::new()));
}

