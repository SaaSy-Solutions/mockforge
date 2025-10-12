use semver::{Version, VersionReq};

fn main() {
    let req = VersionReq::parse("1.0.0").unwrap();
    let v1 = Version::parse("1.0.0").unwrap();
    let v2 = Version::parse("1.1.0").unwrap();

    println!("Req: {:?}", req);
    println!("Matches 1.0.0: {}", req.matches(&v1));
    println!("Matches 1.1.0: {}", req.matches(&v2));
}
