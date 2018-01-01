extern crate shadertoy;

#[test]
fn test_query() {
    let client = shadertoy::Client::new("Bd8tWD");

    // try connecting to shadertoy.com and search for all "car" shadertoys
    // this of course requires one to be online
    let query = client.search(Some("car")).unwrap();
    assert!(query.len() > 0);

    // get the first shader in the list

    let shader = client.get_shader(&query[0]).unwrap();
    assert!(shader.renderpass.len() > 0);
}