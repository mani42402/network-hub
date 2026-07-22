//! Mock implementation of [`InterfaceInfo`], with scriptable simulation.
//!
//! Holds a set of interfaces in memory and lets a test mutate them — bring a
//! link up or down, change addresses, rename (F-13), plug and unplug — each of
//! which emits the matching [`InterfaceEvent`] on the watch channel, exactly as a
//! real backend would from a netlink/SCDynamicStore notification.

// Test-support code: the simulate_* methods lock a Mutex and panic on poison,
// which only happens if a test thread already panicked. See mock/fault.rs.
#![allow(clippy::missing_panics_doc)]

use std::collections::BTreeMap;
use std::sync::Mutex;

use async_trait::async_trait;
use tokio::sync::broadcast;

use crate::error::PalResult;
use crate::interface::{
    Interface, InterfaceEvent, InterfaceEventStream, InterfaceId, InterfaceInfo, IpCidr,
};
use crate::mock::fault::FaultInjector;

/// A mock [`InterfaceInfo`] backend.
///
/// Construct it, add interfaces, hand a clone to the code under test as a
/// `dyn InterfaceInfo`, and drive changes from the test with the `simulate_*`
/// methods.
pub struct MockInterfaces {
    state: Mutex<BTreeMap<InterfaceId, Interface>>,
    events: broadcast::Sender<InterfaceEvent>,
    faults: FaultInjector,
}

impl MockInterfaces {
    /// A new backend with no interfaces.
    #[must_use]
    pub fn new() -> Self {
        // A generous buffer; tests never produce enough events to lag it, and a
        // lagging real subscriber resynchronizes from list() anyway (§5.4).
        let (events, _) = broadcast::channel(256);
        Self {
            state: Mutex::new(BTreeMap::new()),
            events,
            faults: FaultInjector::default(),
        }
    }

    /// The fault injector, to arm failures/latency on `list`/`get`.
    #[must_use]
    pub fn faults(&self) -> &FaultInjector {
        &self.faults
    }

    /// Adds an interface and emits [`InterfaceEvent::Added`]. Simulates an
    /// adapter being plugged in or a tunnel being created.
    pub fn simulate_add(&self, interface: Interface) {
        self.state
            .lock()
            .expect("mock state mutex")
            .insert(interface.id.clone(), interface.clone());
        // A send with no live subscribers returns Err; that is fine.
        let _ = self.events.send(InterfaceEvent::Added { interface });
    }

    /// Removes an interface and emits [`InterfaceEvent::Removed`]. Simulates an
    /// adapter being unplugged.
    pub fn simulate_remove(&self, id: &InterfaceId) {
        let existed = self
            .state
            .lock()
            .expect("mock state mutex")
            .remove(id)
            .is_some();
        if existed {
            let _ = self.events.send(InterfaceEvent::Removed { id: id.clone() });
        }
    }

    /// Brings a link up or down and emits [`InterfaceEvent::LinkChanged`].
    pub fn simulate_link(&self, id: &InterfaceId, is_up: bool) {
        let mut state = self.state.lock().expect("mock state mutex");
        if let Some(iface) = state.get_mut(id) {
            iface.is_up = is_up;
            let _ = self.events.send(InterfaceEvent::LinkChanged {
                id: id.clone(),
                is_up,
            });
        }
    }

    /// Replaces an interface's addresses and emits
    /// [`InterfaceEvent::AddressesChanged`]. Simulates a DHCP lease change.
    pub fn simulate_addresses(&self, id: &InterfaceId, addresses: Vec<IpCidr>) {
        let mut state = self.state.lock().expect("mock state mutex");
        if let Some(iface) = state.get_mut(id) {
            iface.addresses.clone_from(&addresses);
            let _ = self.events.send(InterfaceEvent::AddressesChanged {
                id: id.clone(),
                addresses,
            });
        }
    }

    /// Renames an interface while keeping its stable id, emitting
    /// [`InterfaceEvent::Renamed`]. This is the F-13 case: the identity the
    /// daemon keys on is unchanged even though the OS name is different.
    pub fn simulate_rename(&self, id: &InterfaceId, new_name: impl Into<String>) {
        let new_name = new_name.into();
        let mut state = self.state.lock().expect("mock state mutex");
        if let Some(iface) = state.get_mut(id) {
            iface.name.clone_from(&new_name);
            let _ = self.events.send(InterfaceEvent::Renamed {
                id: id.clone(),
                new_name,
            });
        }
    }
}

impl Default for MockInterfaces {
    fn default() -> Self {
        Self::new()
    }
}

impl std::fmt::Debug for MockInterfaces {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let count = self.state.lock().map_or(0, |s| s.len());
        f.debug_struct("MockInterfaces")
            .field("interfaces", &count)
            .finish_non_exhaustive()
    }
}

#[async_trait]
impl InterfaceInfo for MockInterfaces {
    async fn list(&self) -> PalResult<Vec<Interface>> {
        self.faults.check("interface.list").await?;
        Ok(self
            .state
            .lock()
            .expect("mock state mutex")
            .values()
            .cloned()
            .collect())
    }

    async fn get(&self, id: &InterfaceId) -> PalResult<Option<Interface>> {
        self.faults.check("interface.get").await?;
        Ok(self
            .state
            .lock()
            .expect("mock state mutex")
            .get(id)
            .cloned())
    }

    fn watch(&self) -> InterfaceEventStream {
        self.events.subscribe()
    }
}
