extern crate shadertoy;

#[test]
fn test_query() {
    let client = shadertoy::Client::new("Bd8tWD");

    // try connecting to shadertoy.com and search for all "car" shadertoys
    // this of course requires one to be online
    let query = client
        .search(shadertoy::SearchParams {
            string: "car",
            sort_order: shadertoy::SearchSortOrder::Popular,
            filters: vec![shadertoy::SearchFilter::MultiPass],
        })
        .unwrap();

    assert!(query.len() > 0);

    // get the first shader in the list
    let shader = client.get_shader(&query[0]).unwrap();
    assert!(shader.renderpass.len() > 0);

    // try getting a specific shader, this should suceed
    let shader = client.get_shader("4d2BDy");
    assert!(shader.is_ok());

    // try getting a random shader, this should fail
    let shader = client.get_shader("doesnt_exist");
    assert!(shader.is_err());
}

#[test]
fn test_invalid_api_key() {
    let client = shadertoy::Client::new("incorrect");

    // try getting a specific shader, this should fail due to the API key
    let shader = client.get_shader("4d2BDy");
    assert!(shader.is_err());
    println!("error: {}", shader.err().unwrap());
}
