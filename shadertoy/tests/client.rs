extern crate shadertoy;

#[test]
fn test_query() {
    let api_key = "Bd8tWD";
    let client = reqwest::blocking::Client::new();

    // try connecting to shadertoy.com and search for all "car" shadertoys
    // this of course requires one to be online
    let query = shadertoy::SearchQuery {
        string: "car",
        sort_order: shadertoy::SearchSortOrder::Popular,
        filters: vec![shadertoy::SearchFilter::MultiPass],
        api_key,
    }
    .issue(&client)
    .unwrap();

    assert!(!query.is_empty());

    // get the first shader in the list
    let shader = shadertoy::ShaderQuery {
        shader_id: &query[0],
        api_key,
    }
    .issue(&client)
    .unwrap();
    assert!(!shader.renderpass.is_empty());

    // try getting a specific shader, this should suceed
    let shader = shadertoy::ShaderQuery {
        shader_id: "4d2BDy",
        api_key,
    }
    .issue(&client);
    assert!(shader.is_ok());

    // try getting a random shader, this should fail
    let shader = shadertoy::ShaderQuery {
        shader_id: "doesnt_exist",
        api_key,
    }
    .issue(&client);
    assert!(shader.is_err());
}

#[test]
fn test_invalid_api_key() {
    let api_key = "incorrect";
    let client = reqwest::blocking::Client::new();

    // try getting a specific shader, this should fail due to the API key
    let shader = shadertoy::ShaderQuery {
        shader_id: "4d2BDy",
        api_key,
    }
    .issue(&client);

    assert!(shader.is_err());
    println!("error: {}", shader.err().unwrap());
}
