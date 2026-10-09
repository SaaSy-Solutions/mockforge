//! Owned port reservations for sibling test-server subprocesses.
//!
//! Keep real fixture-endpoint listeners while selecting ports, then retain process-local
//! claims across the listener-to-child handoff. Claim checks happen BEFORE any
//! probe binds, so a sibling cannot briefly steal a handed-off port either.
//! Separate processes do not share this registry: eliminating that external
//! race requires passing sockets to the CLI or discovering its bound ports.

use std::collections::HashSet;
use std::io;
use std::net::TcpListener;
use std::sync::atomic::{AtomicU32, Ordering};
use std::sync::{Mutex, MutexGuard, OnceLock};
use std::time::{SystemTime, UNIX_EPOCH};

const FIRST_PORT: u32 = 49152;
const PORT_COUNT: u32 = 65536 - FIRST_PORT;
const MAX_ATTEMPTS: usize = 128;
// Coprime with the power-of-two range: a bounded search samples separated
// candidates, while a full cursor cycle still visits every port exactly once.
const CANDIDATE_STEP: u32 = 257;

fn claims() -> MutexGuard<'static, HashSet<u16>> {
    static CLAIMS: OnceLock<Mutex<HashSet<u16>>> = OnceLock::new();
    CLAIMS
        .get_or_init(|| Mutex::new(HashSet::new()))
        .lock()
        .unwrap_or_else(|poisoned| poisoned.into_inner())
}

fn candidate_port(sequence: u32) -> u16 {
    (FIRST_PORT + sequence % PORT_COUNT) as u16
}

fn next_port() -> u16 {
    static CURSOR: OnceLock<AtomicU32> = OnceLock::new();
    let cursor = CURSOR.get_or_init(|| {
        // Distribution only, not a security secret or random capability.
        let time = SystemTime::now().duration_since(UNIX_EPOCH).map_or(0, |d| d.subsec_nanos());
        AtomicU32::new(time ^ std::process::id())
    });
    candidate_port(cursor.fetch_add(CANDIDATE_STEP, Ordering::Relaxed))
}

fn windows_candidate_denial(error: &io::Error) -> bool {
    // Winsock bind can report WSAEACCES for an exclusively owned address.
    // This rejects one auto-selected candidate; it does not relax permissions.
    // Other permission errors, and this number on non-Windows, stay fatal.
    cfg!(windows) && error.raw_os_error() == Some(10013)
}

fn bind_endpoints(port: u16) -> io::Result<Vec<TcpListener>> {
    let wildcard = TcpListener::bind(("0.0.0.0", port))?;
    #[cfg(windows)]
    {
        // Windows permits a specific-address listener alongside a wildcard
        // listener. If loopback is already occupied, `?` drops only our new
        // wildcard; it never closes the existing listener or claims the port.
        let loopback = TcpListener::bind(("127.0.0.1", port))?;
        Ok(vec![wildcard, loopback])
    }
    #[cfg(not(windows))]
    {
        // Non-Windows keeps the wildcard listener. The direct endpoint tests
        // must qualify its occupancy semantics on each supported target.
        Ok(vec![wildcard])
    }
}

#[cfg(test)]
pub(crate) fn test_guard() -> MutexGuard<'static, ()> {
    static TESTS: Mutex<()> = Mutex::new(());
    TESTS.lock().unwrap_or_else(|poisoned| poisoned.into_inner())
}

#[cfg(test)]
pub(crate) fn is_claimed(port: u16) -> bool {
    claims().contains(&port)
}

/// A temporary listener plus a claim owned until the child has exited.
pub(crate) struct PortReservation {
    port: u16,
    listeners: Vec<TcpListener>,
    release_claim: bool,
}

impl PortReservation {
    pub(crate) fn new() -> io::Result<Self> {
        // HTTP/WS use wildcard IPv4; admin uses IPv4 loopback. Windows permits
        // these endpoints to coexist, so reserve both there. Never bind(0): a
        // short probe could steal a claimed port during another child's handoff.
        Self::reserve_with(next_port, bind_endpoints)
    }

    fn reserve_with(
        mut candidate: impl FnMut() -> u16,
        mut bind: impl FnMut(u16) -> io::Result<Vec<TcpListener>>,
    ) -> io::Result<Self> {
        let mut denied_candidate = None;
        for _ in 0..MAX_ATTEMPTS {
            let port = candidate();
            if port == 0 {
                return Err(io::Error::new(
                    io::ErrorKind::InvalidInput,
                    "port zero is not a candidate",
                ));
            }
            // Only the short exact-port probe is locked. No guard spans child
            // spawn, health checks, or the lifetime of a returned reservation.
            let mut owned = claims();
            if owned.contains(&port) {
                continue;
            }
            let listeners = match bind(port) {
                Ok(listeners) => listeners,
                Err(error) if error.kind() == io::ErrorKind::AddrInUse => continue,
                Err(error) if windows_candidate_denial(&error) => {
                    // Preserve the original native denial even if subsequent
                    // candidates only collide. Exhaustion must remain failure.
                    denied_candidate.get_or_insert(error);
                    continue;
                }
                Err(error) => return Err(error),
            };
            if listeners.is_empty() {
                return Err(io::Error::new(
                    io::ErrorKind::InvalidInput,
                    "binder returned no listeners",
                ));
            }
            for listener in &listeners {
                if listener.local_addr()?.port() != port {
                    return Err(io::Error::new(
                        io::ErrorKind::InvalidInput,
                        "binder changed the requested port",
                    ));
                }
            }
            owned.insert(port);
            return Ok(Self {
                port,
                listeners,
                release_claim: true,
            });
        }
        Err(denied_candidate.unwrap_or_else(|| {
            io::Error::new(
                io::ErrorKind::AddrInUse,
                "could not reserve an unclaimed test-server port",
            )
        }))
    }

    pub(crate) fn port(&self) -> u16 {
        self.port
    }

    pub(crate) fn handoff(&mut self) {
        self.listeners.clear();
    }

    pub(crate) fn retain_claim(&mut self) {
        self.release_claim = false;
    }
}

impl Drop for PortReservation {
    fn drop(&mut self) {
        self.listeners.clear();
        if self.release_claim {
            claims().remove(&self.port);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::thread;

    fn exact(port: u16) -> io::Result<Vec<TcpListener>> {
        bind_endpoints(port)
    }

    #[test]
    fn reservation_holds_fixture_ipv4_endpoints() {
        let _guard = test_guard();
        let lease = PortReservation::new().expect("reserve port");
        assert_ne!(lease.port(), 0);
        assert!(TcpListener::bind(("0.0.0.0", lease.port())).is_err());
        assert!(TcpListener::bind(("127.0.0.1", lease.port())).is_err());
    }

    #[test]
    fn handoff_releases_socket_but_keeps_claim() {
        let _guard = test_guard();
        let mut lease = PortReservation::new().expect("reserve port");
        lease.handoff();
        let child = exact(lease.port()).expect("child binds");
        assert!(is_claimed(lease.port()));
        drop(child);
    }

    #[test]
    fn handed_off_claim_is_never_probed() {
        let _guard = test_guard();
        let mut owner = PortReservation::new().expect("reserve owner port");
        owner.handoff();
        let mut selections = 0;
        let mut probes = 0;
        let result = PortReservation::reserve_with(
            || {
                selections += 1;
                owner.port()
            },
            |port| {
                probes += 1;
                let probe = exact(port)?;
                // Deterministic regression control: if a claim is checked only
                // after binding, the child's concurrent bind fails right here.
                let child = exact(owner.port());
                assert!(child.is_ok(), "a sibling probe stole the child's claimed port");
                Ok(probe)
            },
        );
        assert!(matches!(result, Err(ref e) if e.kind() == io::ErrorKind::AddrInUse));
        assert_eq!(selections, MAX_ATTEMPTS);
        assert_eq!(probes, 0, "known claims must never reach the OS binder");
        assert!(exact(owner.port()).is_ok(), "child can still bind its handoff port");
    }

    #[test]
    fn claimed_candidate_advances_without_binding() {
        let _guard = test_guard();
        let mut owner = PortReservation::new().expect("reserve owner port");
        owner.handoff();
        let mut selections = 0;
        let sibling = PortReservation::reserve_with(
            || {
                selections += 1;
                if selections == 1 {
                    owner.port()
                } else {
                    next_port()
                }
            },
            |port| {
                assert_ne!(port, owner.port(), "claimed candidate reached binder");
                exact(port)
            },
        )
        .expect("reserve different port");
        assert_ne!(owner.port(), sibling.port());
        assert!(selections >= 2);
    }

    #[test]
    fn occupied_wildcard_candidate_advances() {
        let _guard = test_guard();
        let occupied = TcpListener::bind("0.0.0.0:0").expect("external occupied socket");
        let port = occupied.local_addr().expect("occupied address").port();
        let mut selections = 0;
        let lease = PortReservation::reserve_with(
            || {
                selections += 1;
                if selections == 1 {
                    port
                } else {
                    next_port()
                }
            },
            exact,
        )
        .expect("skip occupied candidate");
        assert!(selections >= 2);
        assert_ne!(lease.port(), port);
    }

    #[test]
    fn occupied_loopback_candidate_advances() {
        let _guard = test_guard();
        let occupied = TcpListener::bind("127.0.0.1:0").expect("existing admin listener");
        let port = occupied.local_addr().expect("admin address").port();
        let mut selections = 0;
        let lease = PortReservation::reserve_with(
            || {
                selections += 1;
                if selections == 1 {
                    port
                } else {
                    next_port()
                }
            },
            exact,
        )
        .expect("skip occupied admin endpoint");
        assert!(selections >= 2);
        assert_ne!(lease.port(), port);
        assert!(!is_claimed(port), "failed partial reservation must not claim the port");
        assert!(TcpListener::bind(("127.0.0.1", port)).is_err(), "existing admin stays bound");
    }

    #[cfg(windows)]
    #[test]
    fn failed_loopback_probe_drops_only_its_wildcard() {
        let _guard = test_guard();
        let occupied = TcpListener::bind("127.0.0.1:0").expect("existing admin listener");
        let port = occupied.local_addr().expect("admin address").port();
        assert!(bind_endpoints(port).is_err(), "occupied loopback must reject reservation");
        let wildcard =
            TcpListener::bind(("0.0.0.0", port)).expect("temporary wildcard was dropped");
        assert!(TcpListener::bind(("127.0.0.1", port)).is_err(), "existing admin remains owned");
        drop(wildcard);
        drop(occupied);
        assert!(bind_endpoints(port).is_ok(), "both endpoints reusable after owner closes");
    }

    #[test]
    fn cursor_wrap_stays_in_high_port_range() {
        assert_eq!(candidate_port(0), 49152);
        assert_eq!(candidate_port(PORT_COUNT - 1), 65535);
        assert_eq!(candidate_port(PORT_COUNT), 49152);
        assert_eq!(candidate_port(u32::MAX), 65535);
        assert_eq!(candidate_port(u32::MAX.wrapping_add(1)), 49152);
    }

    #[test]
    fn stride_visits_the_full_range_without_repetition() {
        let ports: HashSet<_> = (0..PORT_COUNT)
            .map(|index| candidate_port(index.wrapping_mul(CANDIDATE_STEP)))
            .collect();
        assert_eq!(ports.len(), PORT_COUNT as usize);
        assert!(ports.contains(&49152));
        assert!(ports.contains(&65535));
        assert_eq!(candidate_port(PORT_COUNT.wrapping_mul(CANDIDATE_STEP)), 49152);
    }

    #[cfg(windows)]
    #[test]
    fn windows_native_denial_advances_to_an_allowed_candidate() {
        let _guard = test_guard();
        let mut attempts = 0;
        let lease = PortReservation::reserve_with(next_port, |port| {
            attempts += 1;
            if attempts == 1 {
                Err(io::Error::from_raw_os_error(10013))
            } else {
                exact(port)
            }
        })
        .expect("reserve an allowed candidate");
        assert!(attempts >= 2);
        assert!(is_claimed(lease.port()));
    }

    #[cfg(windows)]
    #[test]
    fn exhausted_search_preserves_native_denial_even_after_collisions() {
        let _guard = test_guard();
        let mut attempts = 0;
        let result = PortReservation::reserve_with(next_port, |_| {
            attempts += 1;
            if attempts == 1 {
                Err(io::Error::from_raw_os_error(10013))
            } else {
                Err(io::Error::new(io::ErrorKind::AddrInUse, "synthetic collision"))
            }
        });
        assert_eq!(attempts, MAX_ATTEMPTS);
        assert!(matches!(result, Err(ref e) if e.raw_os_error() == Some(10013)));
    }

    #[cfg(not(windows))]
    #[test]
    fn windows_error_number_is_not_retried_on_other_targets() {
        let _guard = test_guard();
        let mut attempts = 0;
        let result = PortReservation::reserve_with(next_port, |_| {
            attempts += 1;
            Err(io::Error::from_raw_os_error(10013))
        });
        assert_eq!(attempts, 1);
        assert!(matches!(result, Err(ref e) if e.raw_os_error() == Some(10013)));
    }

    #[test]
    fn drop_releases_socket_and_claim() {
        let _guard = test_guard();
        let lease = PortReservation::new().expect("reserve port");
        let port = lease.port();
        drop(lease);
        assert!(!is_claimed(port));
        let again = PortReservation::reserve_with(|| port, exact).expect("released port reusable");
        assert_eq!(again.port(), port);
    }

    #[test]
    fn failed_startup_releases_handed_off_claim() {
        let _guard = test_guard();
        fn failed_startup(port: &mut u16) -> io::Result<()> {
            let mut lease = PortReservation::new()?;
            *port = lease.port();
            lease.handoff();
            Err(io::Error::new(io::ErrorKind::NotFound, "synthetic spawn failure"))
        }
        let mut port = 0;
        assert!(failed_startup(&mut port).is_err());
        assert!(!is_claimed(port));
        let again =
            PortReservation::reserve_with(|| port, exact).expect("failed startup released port");
        assert_eq!(again.port(), port);
    }

    #[test]
    fn unconfirmed_cleanup_can_preserve_claim() {
        let _guard = test_guard();
        let mut lease = PortReservation::new().expect("reserve port");
        let port = lease.port();
        lease.handoff();
        lease.retain_claim();
        drop(lease);
        assert!(is_claimed(port));
        // This test owns the synthetic abandoned claim; no child was spawned.
        claims().remove(&port);
    }

    #[test]
    fn parallel_startups_keep_distinct_handed_off_ports() {
        let _guard = test_guard();
        let workers: Vec<_> = (0..8)
            .map(|_| {
                thread::spawn(|| {
                    (0..5)
                        .map(|_| {
                            let mut lease = PortReservation::new()?;
                            lease.handoff();
                            Ok(lease)
                        })
                        .collect::<io::Result<Vec<_>>>()
                })
            })
            .collect();
        // JoinHandle results own the leases until collected. No barrier can
        // strand other workers if one returns an allocation error or panics.
        let results: Vec<_> = workers.into_iter().map(|worker| worker.join()).collect();
        let leases: Vec<_> = results
            .into_iter()
            .flat_map(|result| result.expect("join allocator").expect("parallel allocation"))
            .collect();
        let ports: HashSet<_> = leases.iter().map(PortReservation::port).collect();
        assert_eq!(ports.len(), 40, "all parallel claims must be unique");
    }

    #[test]
    fn bind_error_is_not_hidden() {
        let _guard = test_guard();
        let mut attempts = 0;
        let result = PortReservation::reserve_with(next_port, |_| {
            attempts += 1;
            Err(io::Error::new(io::ErrorKind::PermissionDenied, "synthetic bind error"))
        });
        assert_eq!(attempts, 1, "generic permission failures must not be retried");
        assert!(matches!(result, Err(ref e) if e.kind() == io::ErrorKind::PermissionDenied));
    }
}
