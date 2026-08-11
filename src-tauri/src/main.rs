fn main() {
    if std::env::args().nth(1).as_deref() == Some("--self-test") {
        run_package_self_test();
    } else {
        notes_planner_desktop::run();
    }
}

fn run_package_self_test() {
    let data_home = std::env::var_os("XDG_DATA_HOME")
        .map(std::path::PathBuf::from)
        .or_else(|| {
            std::env::var_os("HOME")
                .map(std::path::PathBuf::from)
                .map(|home| home.join(".local/share"))
        })
        .unwrap_or_else(|| std::path::PathBuf::from(".local/share"));
    let app_data_directory = data_home.join("com.narguis.notes.desktop");
    match notes_planner_desktop::run_package_self_test(&app_data_directory) {
        Ok(()) => println!("{}", notes_planner_desktop::PACKAGE_SELF_TEST_SUCCESS_LOG),
        Err(error) => {
            eprintln!("package self-test failed: {error}");
            std::process::exit(1);
        }
    }
}
