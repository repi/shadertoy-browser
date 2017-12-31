extern crate shadertoy;

#[test]
fn test_query() {
    let client = shadertoy::Client::new("Bd8tWD");

    // try connecting to shadertoy.com and search for all "car" shadertoys
    // this of course requires one to be online
    let result = client.search(Some("car"));

    assert!(result.is_ok());
    assert!(result.ok().unwrap().len() > 0);
}
