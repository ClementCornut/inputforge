use dioxus::prelude::*;

use crate::components::{
    Badge, BadgeVariant, Icon, InputSize, MenuItem, MenuItems, MenuRoot, MenuTrigger, TextInput,
};
use crate::context::AppContext;
use crate::context::{ProfileRowOrigin, ProfileRowView};
use crate::frame::profiles::actions::{
    NewProfileValidationError, create_manual_snapshot_action, profile_delete_action,
    profile_duplicate_action, profile_open_action, profile_rename_action, profile_reveal_action,
    validate_rename,
};
use crate::frame::profiles::new_profile::add_external_to_library_command;
use crate::frame::profiles::projection::project_profile_rows;
use crate::frame::view_state::ViewState;
use crate::icons::Icon as IconKind;

#[component]
pub(crate) fn ProfileLibrary(rows: Vec<ProfileRowView>, active_id: String) -> Element {
    let ctx = use_context::<AppContext>();
    let view = use_context::<ViewState>();
    let filter = view.profiles_panel.read().filter.clone();
    let projected = project_profile_rows(&rows, &active_id, &filter);
    let mut rename_profile = use_signal(|| None::<String>);
    let mut rename_value = use_signal(String::new);
    let rename_value_read: ReadSignal<String> = rename_value.into();
    let mut rename_error = use_signal(|| None::<NewProfileValidationError>);

    // Existing names for inline rename collision checks. Sourced from
    // the projected library list so we match exactly what the user sees.
    let existing_names: Vec<String> = rows.iter().map(|r| r.name.clone()).collect();

    let inactive_visible = projected.iter().any(|row| !row.is_active);
    let filter_active = !filter.trim().is_empty();
    let show_filtered_empty = filter_active && !inactive_visible;

    let view_for_memo = view;
    let filter_memo = use_memo(move || view_for_memo.profiles_panel.read().filter.clone());
    let filter_read: ReadSignal<String> = filter_memo.into();
    let mut view_for_filter = view;

    rsx! {
        div { class: "profiles-panel__library",
            div { class: "profiles-panel__filter",
                TextInput {
                    value: filter_read,
                    size: InputSize::Sm,
                    placeholder: "Filter profiles".to_owned(),
                    oninput: move |evt: FormEvent| {
                        let value = evt.value();
                        view_for_filter.profiles_panel.write().filter = value;
                    },
                }
            }
            for row in projected {
                {
                    let commands = ctx.commands.clone();
                    let add_path = std::path::PathBuf::from(row.id.clone());
                    let add_name = row.name.clone();
                    let add_existing = existing_names.clone();
                    let add_click = move |_| {
                        if let Ok(command) = add_external_to_library_command(
                            add_path.clone(),
                            &add_name,
                            &add_existing,
                        ) {
                            let _ = commands.send(command);
                        }
                    };
                    let commands = ctx.commands.clone();
                    let open_path = std::path::PathBuf::from(row.id.clone());
                    let open_click = move |_| {
                        let _ = commands.send(profile_open_action(open_path.clone()));
                    };
                    let commands = ctx.commands.clone();
                    let reveal_path = std::path::PathBuf::from(row.id.clone());
                    let reveal_click = move |_| {
                        let _ = commands.send(profile_reveal_action(reveal_path.clone()));
                    };
                    let commands = ctx.commands.clone();
                    let duplicate_path = std::path::PathBuf::from(row.id.clone());
                    let duplicate_name = format!("{} Copy", row.name);
                    let duplicate_click = move |_| {
                        if let Some(command) = profile_duplicate_action(duplicate_path.clone(), &duplicate_name) {
                            let _ = commands.send(command);
                        }
                    };
                    let commands = ctx.commands.clone();
                    let delete_name = row.name.clone();
                    let delete_click = move |_| {
                        let _ = commands.send(profile_delete_action(&delete_name).command);
                    };
                    let commands = ctx.commands.clone();
                    let snapshot_click = move |_| {
                        let _ = commands.send(create_manual_snapshot_action());
                    };
                    let row_name_for_rename = row.name.clone();
                    let rename_click = move |_| {
                        rename_value.set(row_name_for_rename.clone());
                        rename_profile.set(Some(row_name_for_rename.clone()));
                        rename_error.set(None);
                    };
                    let commands_keydown = ctx.commands.clone();
                    let rename_from_keydown = row.name.clone();
                    let rename_existing_keydown = existing_names.clone();
                    let rename_keydown = move |evt: KeyboardEvent| match evt.key() {
                        Key::Enter => {
                            evt.prevent_default();
                            let next = rename_value.read().clone();
                            match validate_rename(
                                &rename_from_keydown,
                                &next,
                                &rename_existing_keydown,
                            ) {
                                Ok(validated) => {
                                    if let Some(command) = profile_rename_action(
                                        &rename_from_keydown,
                                        &validated,
                                    ) {
                                        let _ = commands_keydown.send(command);
                                    }
                                    rename_error.set(None);
                                    rename_profile.set(None);
                                }
                                Err(err) => rename_error.set(Some(err)),
                            }
                        }
                        Key::Escape => {
                            evt.prevent_default();
                            rename_error.set(None);
                            rename_profile.set(None);
                        }
                        _ => {}
                    };
                    // Blur commits silently. Errors are not surfaced
                    // here because the user is leaving the field, so
                    // a no-op fallback is preferable to a stuck banner.
                    let commands_blur = ctx.commands.clone();
                    let rename_from_blur = row.name.clone();
                    let rename_existing_blur = existing_names.clone();
                    let rename_blur = move |_| {
                        let next = rename_value.read().clone();
                        if let Ok(validated) = validate_rename(
                            &rename_from_blur,
                            &next,
                            &rename_existing_blur,
                        ) {
                            if let Some(command) = profile_rename_action(&rename_from_blur, &validated)
                            {
                                let _ = commands_blur.send(command);
                            }
                        }
                        rename_error.set(None);
                        rename_profile.set(None);
                    };
                    let rename_input_oninput = move |evt: FormEvent| {
                        rename_value.set(evt.value());
                        if rename_error.read().is_some() {
                            rename_error.set(None);
                        }
                    };
                    let row_is_renaming = rename_profile.read().as_deref() == Some(row.name.as_str());
                    let row_rename_error_msg = if row_is_renaming {
                        rename_error
                            .read()
                            .as_ref()
                            .map(NewProfileValidationError::user_message)
                    } else {
                        None
                    };
                    let mode_label = if row.mode_count == 1 {
                        "1 mode".to_owned()
                    } else {
                        format!("{} modes", row.mode_count)
                    };
                    let last_edited_label = row.last_edited_label.clone();
                    rsx! {
                article {
                    class: if row.is_active { "profile-row profile-row--active" } else { "profile-row" },
                    "data-profile-id": "{row.id}",
                    div { class: "profile-row__main",
                        div { class: "profile-row__title",
                            strong { "{row.name}" }
                            if row.is_active {
                                Badge { variant: BadgeVariant::Success, "Active" }
                            }
                            if matches!(row.origin, ProfileRowOrigin::External) {
                                Badge { variant: BadgeVariant::Info, "External" }
                            }
                        }
                        div { class: "profile-row__meta",
                            span { class: "profile-row__mode-count", "{mode_label}" }
                            if let Some(last) = last_edited_label {
                                span { class: "profile-row__sep", "·" }
                                span { class: "profile-row__last-edited", "{last}" }
                            }
                        }
                        if row_is_renaming {
                            div { class: "profile-row__rename",
                                TextInput {
                                    value: rename_value_read,
                                    size: InputSize::Sm,
                                    placeholder: "Profile name".to_owned(),
                                    oninput: rename_input_oninput,
                                    onmounted: move |evt: MountedEvent| {
                                        spawn(async move {
                                            let _ = evt.set_focus(true).await;
                                        });
                                    },
                                    onkeydown: rename_keydown,
                                    onblur: rename_blur,
                                }
                            }
                            if let Some(message) = row_rename_error_msg.clone() {
                                div { class: "profile-row__rename-error", "{message}" }
                            }
                        }
                    }
                    MenuRoot { class: "profile-row__menu".to_owned(),
                        MenuTrigger { class: "profile-row__menu-trigger".to_owned(),
                            Icon { name: IconKind::DotsVertical }
                            span { class: "profile-row__menu-label", "Profile actions for {row.name}" }
                        }
                        MenuItems {
                            if row.can_open && !row.is_active { MenuItem { onclick: open_click, "Open" } }
                            if row.can_add_to_library { MenuItem { onclick: add_click, "Add to library" } }
                            if row.can_snapshot_now { MenuItem { onclick: snapshot_click, "Snapshot now" } }
                            if row.can_rename { MenuItem { onclick: rename_click, "Rename" } }
                            if row.can_duplicate { MenuItem { onclick: duplicate_click, "Duplicate" } }
                            if row.can_reveal { MenuItem { onclick: reveal_click, "Reveal" } }
                            if row.can_delete { MenuItem { onclick: delete_click, class: "profile-row__danger-item".to_owned(), "Delete" } }
                        }
                    }
                }
                    }
                }
            }
            if show_filtered_empty {
                div { class: "profile-row__filtered-empty",
                    "No matching profiles."
                }
            }
        }
    }
}
