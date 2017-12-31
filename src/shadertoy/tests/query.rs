extern crate shadertoy;

#[test]
fn test_query() {
    let service = shadertoy::Service::new("Bd8tWD");

    // try connecting to shadertoy.com and search for all "car" shadertoys
    // this of course requires one to be online
    let result = service.search(Some("car"));

    assert!(result.is_ok());
    assert!(result.ok().unwrap().len() > 0);
}
