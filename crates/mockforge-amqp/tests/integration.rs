use mockforge_amqp::fixtures::AmqpFixture;
use std::path::PathBuf;

#[tokio::test]
async fn test_fixture_loading() {
    let fixtures_dir = PathBuf::from("../../fixtures/amqp");
    let fixtures = AmqpFixture::load_from_dir(&fixtures_dir).unwrap();

    assert!(!fixtures.is_empty(), "Should load at least one fixture");

    let order_fixture = fixtures.iter().find(|f| f.identifier == "order-processing").unwrap();
    assert_eq!(order_fixture.name, "Order Processing Workflow");
    assert_eq!(order_fixture.exchanges.len(), 2);
    assert_eq!(order_fixture.queues.len(), 3);
    assert_eq!(order_fixture.bindings.len(), 3);
    assert!(order_fixture.auto_publish.is_some());
}