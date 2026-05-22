#[test]
fn readme_states_terminal_ssh_intranet_purpose_and_crs_link() {
    let readme = include_str!("../README.md");
    assert!(
        readme.starts_with(
            "# crstop\n\n`crstop` is designed for checking usage from a terminal when"
        )
    );
    assert!(readme.contains("SSH-accessible intranet relay"));
    assert!(readme.contains("If you are already inside the intranet and can open the CRS web UI, the browser dashboard is still the simplest option."));
    assert!(readme.contains("https://github.com/Wei-Shaw/claude-relay-service"));
}

#[test]
fn readme_stays_concise() {
    let readme = include_str!("../README.md");
    assert!(readme.lines().count() <= 45);
}
