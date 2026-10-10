use oktz_plugin_api::{Ctx, Reply, find, plugin};

#[plugin(name = "ping", alias = ["p"], category = "main", cooldown = 5, is_private = true)]
fn ping(_ctx: Ctx) -> Reply {
    Reply::Text("pong".into())
}

#[plugin(name = "echo", category = "tools", is_owner = true, energi = 3)]
fn echo(ctx: Ctx) -> Reply {
    Reply::Text(ctx.text)
}

#[test]
fn registry_collects_declared_plugins() {
    let names: Vec<&str> = oktz_plugin_api::all_plugins()
        .iter()
        .map(|p| p.meta.name.as_str())
        .collect();
    assert!(names.contains(&"ping"), "registered: {names:?}");
    assert!(names.contains(&"echo"), "registered: {names:?}");
}

#[test]
fn find_resolves_alias() {
    let p = find("p").expect("alias harus ketemu");
    assert_eq!(p.meta.name, "ping");
    assert_eq!(p.meta.cooldown, 5);
}

#[test]
fn macro_populates_meta_fields() {
    let ping = find("ping").unwrap();
    assert_eq!(ping.meta.category, "main");
    assert!(ping.meta.is_private);
    assert!(!ping.meta.is_owner);

    let echo = find("echo").unwrap();
    assert!(echo.meta.is_owner);
    assert_eq!(echo.meta.energi, 3);
}

#[tokio::test]
async fn handler_is_callable() {
    let ping = find("ping").unwrap();
    let reply = (ping.handler)(Ctx {
        text: "x".into(),
        args: vec![],
        sender: "628".into(),
        is_owner: true,
        uptime_secs: 1,
    });
    match reply {
        Reply::Text(t) => assert_eq!(t, "pong"),
        Reply::None => panic!("harus Text"),
    }
}

#[test]
fn unknown_command_returns_none() {
    assert!(find("command-yang-tidak-ada").is_none());
}
