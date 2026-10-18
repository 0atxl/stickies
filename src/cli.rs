use gtk::{gio, glib, prelude::*};

pub const ACTIONS: [(&str, &str); 4] = [
    ("new-note", "New note"),
    ("search", "Search notes"),
    ("toggle-strip", "Toggle edge strip"),
    ("show-all-notes", "All Notes"),
];

pub fn add_options(application: &gio::Application) {
    for (name, description) in ACTIONS {
        application.add_main_option(
            name,
            0u8.into(),
            glib::OptionFlags::NONE,
            glib::OptionArg::None,
            description,
            None,
        );
    }
}

pub fn requested_action(options: &glib::VariantDict) -> Result<Option<&'static str>, &'static str> {
    let mut actions = ACTIONS
        .iter()
        .map(|(name, _)| *name)
        .filter(|name| options.contains(name));
    let action = actions.next();
    if actions.next().is_some() {
        return Err("Use only one action flag per invocation.");
    }
    Ok(action)
}

#[cfg(test)]
mod tests {
    use super::*;
    #[cfg(not(target_os = "windows"))]
    use std::{cell::RefCell, ops::ControlFlow, rc::Rc};

    #[test]
    fn gio_parses_action_flags_and_rejects_unknown_or_combined_actions() {
        #[cfg(not(target_os = "windows"))]
        {
            let parse = |arguments: &[&str]| {
                let application = gio::Application::builder()
                    .flags(
                        gio::ApplicationFlags::NON_UNIQUE
                            | gio::ApplicationFlags::HANDLES_COMMAND_LINE,
                    )
                    .build();
                add_options(&application);
                let parsed = Rc::new(RefCell::new(None));
                let result = parsed.clone();
                application.connect_handle_local_options(move |_, options| {
                    let action = requested_action(options);
                    let status = if action.is_ok() {
                        glib::ExitCode::SUCCESS
                    } else {
                        glib::ExitCode::FAILURE
                    };
                    result.replace(Some(action));
                    ControlFlow::Break(status)
                });
                let status = application.run_with_args(arguments);
                let action = parsed.borrow().as_ref().copied();
                (status, action)
            };
            assert_eq!(
                parse(&["stickies"]),
                (glib::ExitCode::SUCCESS, Some(Ok(None)))
            );
            for (action, _) in ACTIONS {
                let flag = format!("--{action}");
                assert_eq!(
                    parse(&["stickies", &flag]),
                    (glib::ExitCode::SUCCESS, Some(Ok(Some(action))))
                );
            }
            assert_eq!(
                parse(&["stickies", "--search", "--search"]),
                (glib::ExitCode::SUCCESS, Some(Ok(Some("search"))))
            );
            assert_eq!(
                parse(&["stickies", "--new-note", "--search"]),
                (
                    glib::ExitCode::FAILURE,
                    Some(Err("Use only one action flag per invocation."))
                )
            );
            assert_eq!(
                parse(&["stickies", "--unknown"]),
                (glib::ExitCode::FAILURE, None)
            );
        }

        // On Windows, GLib's g_application_run ignores the argv slice and calls
        // g_win32_get_command_line() (reading the cargo test runner's command line).
        // Verify add_options + requested_action directly against VariantDict.
        #[cfg(target_os = "windows")]
        {
            let application = gio::Application::builder()
                .flags(
                    gio::ApplicationFlags::NON_UNIQUE | gio::ApplicationFlags::HANDLES_COMMAND_LINE,
                )
                .build();
            add_options(&application);

            let empty = glib::VariantDict::new(None);
            assert_eq!(requested_action(&empty), Ok(None));

            for (action, _) in ACTIONS {
                let dict = glib::VariantDict::new(None);
                dict.insert(action, true);
                assert_eq!(requested_action(&dict), Ok(Some(action)));
            }

            let combined = glib::VariantDict::new(None);
            combined.insert("new-note", true);
            combined.insert("search", true);
            assert_eq!(
                requested_action(&combined),
                Err("Use only one action flag per invocation.")
            );
        }
    }
}
