//! Exit-criteria tests for `pal-mock` (WP-M0.3).
//!
//! Proves the mock can simulate the scenarios the fault-injection and reconciler
//! suites are built on: interface up/down, address change, rename (F-13), and —
//! the important one — an operation that fails after partially mutating state,
//! whose rollback restores the machine exactly (AD-5).

#![cfg(feature = "mock")]
#![allow(clippy::unwrap_used)]

use std::net::{IpAddr, Ipv4Addr};

use nh_pal::guard::Transaction;
use nh_pal::interface::{
    Interface, InterfaceEvent, InterfaceId, InterfaceInfo, InterfaceKind, IpCidr,
};
use nh_pal::mock::{FaultKind, MockInterfaces, MockRouting, RoutingState};
use nh_pal::routing::{IpFamily, NatSpec, RoutingNat};

fn wifi(id: &str, name: &str) -> Interface {
    Interface {
        id: InterfaceId(id.to_owned()),
        name: name.to_owned(),
        kind: InterfaceKind::Wifi,
        is_up: true,
        mac: Some("a1:b2:c3:d4:e5:f6".parse().unwrap()),
        addresses: vec![],
    }
}

fn cidr(a: u8, b: u8, c: u8, d: u8, prefix: u8) -> IpCidr {
    IpCidr {
        addr: IpAddr::V4(Ipv4Addr::new(a, b, c, d)),
        prefix_len: prefix,
    }
}

#[tokio::test]
async fn lists_added_interfaces() {
    let mock = MockInterfaces::new();
    assert!(mock.list().await.unwrap().is_empty());

    mock.simulate_add(wifi("wlan-stable-1", "wlan0"));
    let list = mock.list().await.unwrap();
    assert_eq!(list.len(), 1);
    assert_eq!(list[0].name, "wlan0");
}

#[tokio::test]
async fn link_up_down_emits_events() {
    let mock = MockInterfaces::new();
    let id = InterfaceId("wlan-stable-1".to_owned());
    mock.simulate_add(wifi("wlan-stable-1", "wlan0"));

    let mut events = mock.watch();
    mock.simulate_link(&id, false);

    let ev = events.recv().await.unwrap();
    assert_eq!(
        ev,
        InterfaceEvent::LinkChanged {
            id: id.clone(),
            is_up: false
        }
    );
    // And the state reflects it.
    assert!(!mock.get(&id).await.unwrap().unwrap().is_up);
}

#[tokio::test]
async fn address_change_emits_event_and_updates_state() {
    let mock = MockInterfaces::new();
    let id = InterfaceId("wlan-stable-1".to_owned());
    mock.simulate_add(wifi("wlan-stable-1", "wlan0"));

    let mut events = mock.watch();
    let new_addrs = vec![cidr(192, 168, 1, 5, 24)];
    mock.simulate_addresses(&id, new_addrs.clone());

    match events.recv().await.unwrap() {
        InterfaceEvent::AddressesChanged { id: got, addresses } => {
            assert_eq!(got, id);
            assert_eq!(addresses, new_addrs);
        }
        other => panic!("expected AddressesChanged, got {other:?}"),
    }
    assert_eq!(mock.get(&id).await.unwrap().unwrap().addresses, new_addrs);
}

#[tokio::test]
async fn rename_keeps_stable_id_f13() {
    // F-13: the OS name changes but the stable id — what the daemon keys on —
    // does not, so nothing bound to the interface breaks.
    let mock = MockInterfaces::new();
    let id = InterfaceId("wlan-stable-1".to_owned());
    mock.simulate_add(wifi("wlan-stable-1", "wlan0"));

    let mut events = mock.watch();
    mock.simulate_rename(&id, "wlp3s0");

    match events.recv().await.unwrap() {
        InterfaceEvent::Renamed { id: got, new_name } => {
            assert_eq!(got, id, "stable id must be unchanged");
            assert_eq!(new_name, "wlp3s0");
        }
        other => panic!("expected Renamed, got {other:?}"),
    }
    // Same id resolves; the name is the new one.
    let iface = mock.get(&id).await.unwrap().unwrap();
    assert_eq!(iface.id, id);
    assert_eq!(iface.name, "wlp3s0");
}

#[tokio::test]
async fn unplug_emits_removed_and_drops_from_list() {
    let mock = MockInterfaces::new();
    let id = InterfaceId("usb-eth-1".to_owned());
    mock.simulate_add(wifi("usb-eth-1", "eth1"));

    let mut events = mock.watch();
    mock.simulate_remove(&id);

    assert_eq!(
        events.recv().await.unwrap(),
        InterfaceEvent::Removed { id: id.clone() }
    );
    assert!(mock.get(&id).await.unwrap().is_none());
    assert!(mock.list().await.unwrap().is_empty());
}

#[tokio::test]
async fn injected_fault_makes_an_operation_fail() {
    let mock = MockInterfaces::new();
    mock.faults()
        .fail_ops_matching("interface.list", FaultKind::Io);
    assert!(
        mock.list().await.is_err(),
        "armed fault should make list() fail"
    );
}

#[tokio::test]
async fn partial_failure_rolls_back_to_original_state() {
    // The AD-5 scenario at the PAL level: a compound operation applies
    // forwarding (step 1) then NAT (step 2). Arm the SECOND mutating operation
    // to fail. The transaction rolls back the acquired step, and the machine is
    // exactly as it started.
    let routing = MockRouting::new();
    assert_eq!(routing.snapshot(), RoutingState::default(), "starts clean");

    // Fail the 2nd mutating op (add_nat).
    routing.faults().fail_after_n(2, FaultKind::Conflict);

    let nat = NatSpec {
        internal: InterfaceId("hotspot0".to_owned()),
        upstream: InterfaceId("wlan0".to_owned()),
    };

    let mut txn = Transaction::new();
    let mut failed = false;

    // Step 1: enable forwarding — succeeds, guard recorded.
    match routing.enable_forwarding(IpFamily::V4).await {
        Ok(guard) => txn.push(guard),
        Err(e) => panic!("step 1 should succeed: {e}"),
    }
    // After step 1, forwarding is on.
    assert!(routing.snapshot().forwarding_v4);

    // Step 2: add NAT — armed to fail.
    match routing.add_nat(&nat).await {
        Ok(guard) => txn.push(guard),
        Err(_) => failed = true,
    }
    assert!(failed, "step 2 was armed to fail");

    // Roll back the acquired steps.
    let errors = txn.rollback().await;
    assert!(errors.is_empty(), "rollback itself should be clean");

    // The machine is byte-for-byte as it began: forwarding restored to false,
    // no NAT rules. This is the invariant the whole product depends on.
    assert_eq!(
        routing.snapshot(),
        RoutingState::default(),
        "after rollback the state must equal the original"
    );
}

#[tokio::test]
async fn committed_transaction_keeps_state() {
    // The success path: both steps apply, the transaction commits, and the state
    // persists (it is now desired state).
    let routing = MockRouting::new();
    let nat = NatSpec {
        internal: InterfaceId("hotspot0".to_owned()),
        upstream: InterfaceId("wlan0".to_owned()),
    };

    let mut txn = Transaction::new();
    txn.push(routing.enable_forwarding(IpFamily::V4).await.unwrap());
    txn.push(routing.add_nat(&nat).await.unwrap());
    txn.commit();

    let snap = routing.snapshot();
    assert!(snap.forwarding_v4);
    assert_eq!(snap.nats, vec![nat]);
}
