use super::*;

#[test]
fn reserved_namespace_is_normalised() {
    for path in [
        "/__mockforge",
        "/__mockforge/api/mocks",
        "//__mockforge/api/mocks",
        "/__mockforge//api",
        "/%5F_mockforge/api/mocks",
        "/%5f%5fmockforge/api",
        "/%255F_mockforge/api",
        "/__MOCKFORGE/api",
        "/x/../__mockforge/api",
        "/./__mockforge/api",
        "/__mockforge;jsessionid=1/api",
        "\\__mockforge\\api",
    ] {
        assert!(is_reserved_path(path), "{path}");
    }
    for path in [
        "/__mockforgeX/api",
        "/api/__mockforge",
        "/",
        "/health",
        "/__mock",
    ] {
        assert!(!is_reserved_path(path), "{path}");
    }
}

#[test]
fn loopback_is_strict() {
    assert!(is_strict_loopback(Ipv4Addr::LOCALHOST.into()));
    assert!(is_strict_loopback(Ipv4Addr::new(127, 1, 2, 3).into()));
    assert!(is_strict_loopback(Ipv6Addr::LOCALHOST.into()));
    for ip in [
        IpAddr::from(Ipv4Addr::UNSPECIFIED),
        IpAddr::from(Ipv6Addr::UNSPECIFIED),
        IpAddr::from(Ipv4Addr::LOCALHOST.to_ipv6_mapped()),
        IpAddr::from(Ipv4Addr::new(10, 0, 0, 1)),
        "fdaa::1".parse().unwrap(),
    ] {
        assert!(!is_strict_loopback(ip), "{ip}");
    }
}

#[test]
fn insecure_opt_out_is_refused_off_loopback() {
    for bind in ["0.0.0.0", "::", "::ffff:127.0.0.1", "10.1.2.3", "fdaa::3"] {
        let bind: IpAddr = bind.parse().unwrap();
        assert_eq!(
            ControlPlanePolicy::from_values(bind, None, Some("1"), None).unwrap_err(),
            ControlPlaneConfigError::InsecureOnNonLoopback(bind),
        );
        // Even with a token set, the opt-out off loopback fails loudly.
        assert!(ControlPlanePolicy::from_values(bind, Some(TOKEN), Some("true"), None).is_err());
        assert!(ControlPlanePolicy::unauthenticated_loopback(bind).is_err());
    }
    for bind in ["127.0.0.1", "::1"] {
        let policy =
            ControlPlanePolicy::from_values(bind.parse().unwrap(), None, Some("1"), None).unwrap();
        assert!(policy.is_unauthenticated());
    }
}

#[test]
fn policy_from_values() {
    let any: IpAddr = Ipv4Addr::UNSPECIFIED.into();
    assert!(ControlPlanePolicy::from_values(any, None, None, None).unwrap().is_locked());
    assert!(ControlPlanePolicy::from_values(any, Some(""), Some("0"), None)
        .unwrap()
        .is_locked());
    assert_eq!(
        ControlPlanePolicy::from_values(any, Some("short"), None, None).unwrap_err(),
        ControlPlaneConfigError::TokenTooShort
    );
    let admin = AdminCredential::new("admin", "a-long-admin-password");
    let policy = ControlPlanePolicy::from_values(any, None, None, admin).unwrap();
    assert!(!policy.is_locked() && !policy.is_unauthenticated());
    // A credential wins over the loopback opt-out.
    let policy =
        ControlPlanePolicy::from_values(Ipv4Addr::LOCALHOST.into(), Some(TOKEN), Some("1"), None)
            .unwrap();
    assert!(!policy.is_unauthenticated());
    assert!(AdminCredential::new("", "x").is_none());
    assert!(AdminCredential::new("x", "").is_none());
}
