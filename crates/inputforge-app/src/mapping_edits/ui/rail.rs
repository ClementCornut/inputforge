//! Rail menu, confirmation, feedback and announcement transitions.
use super::Controller;
use dioxus::prelude::*;
use inputforge_gui_dx::{
    mapping_edits::ui,
    mapping_rail::{Move, Operation, Outcome, Request, Target},
};
impl Controller {
    pub(super) fn rail_ui(&self, mut p: ui::Rail, event: ui::RailEvent) -> bool {
        if p.menu.try_peek().is_err() {
            return false;
        }
        let Some(filtered) = super::filtered(p.filter) else {
            return false;
        };
        let menu_dispatch = matches!(event, ui::RailEvent::MenuDispatch(_));
        let close = match event {
            ui::RailEvent::Close => true,
            ui::RailEvent::Rename(target) => {
                if self.current_target(&self.state.read(), &target).is_none() {
                    p.error
                        .set(Some("This mapping changed. Reopen the action.".into()));
                    return false;
                }
                p.rename.set(Some(target.before.input));
                true
            }
            ui::RailEvent::Duplicate(target) => {
                p.duplicate.set(Some(target));
                true
            }
            ui::RailEvent::BeginDelete(target) => {
                p.delete.set(Some(target));
                true
            }
            ui::RailEvent::Open { mut lookup, x, y } => {
                lookup.filtered = filtered;
                let Some(menu) = self.rail_menu(lookup) else {
                    return false;
                };
                let input = menu.target.before.input.clone();
                p.prepared.set(Some(menu));
                p.menu.set(Some((input, x, y)));
                return true;
            }
            ui::RailEvent::Step { menu, down } => {
                if !(if down { menu.can_down } else { menu.can_up }) {
                    return false;
                }
                return self.rail_ui(
                    p,
                    ui::RailEvent::MenuDispatch(Request {
                        target: menu.target,
                        operation: Operation::Reorder {
                            movement: if down { Move::Down } else { Move::Up },
                            order: menu.order,
                            group: menu.group,
                            filtered,
                        },
                    }),
                );
            }
            ui::RailEvent::Drop {
                mut drag,
                group,
                gap,
            } => {
                let source = drag.peek().clone();
                drag.set(None);
                let Some(menu) = source else { return false };
                return self.rail_ui(
                    p,
                    ui::RailEvent::Dispatch(Request {
                        target: menu.target,
                        operation: Operation::Reorder {
                            movement: Move::Gap(gap),
                            order: menu.order,
                            group,
                            filtered,
                        },
                    }),
                );
            }
            ui::RailEvent::Dispatch(request) | ui::RailEvent::MenuDispatch(request) => {
                return self.rail_feedback(p, request, menu_dispatch);
            }
        };
        if close {
            p.menu.set(None);
            p.prepared.set(None);
        }
        close
    }
    pub(super) fn rail_delete(
        &self,
        mut target: Signal<Option<Target>>,
        mut error: Signal<Option<String>>,
        confirm: bool,
    ) -> bool {
        if target.try_peek().is_err() {
            return false;
        }
        if !confirm {
            target.set(None);
            return true;
        }
        let Some(current) = target.peek().clone() else {
            return false;
        };
        match self.rail(Request {
            target: current,
            operation: Operation::Delete,
        }) {
            Outcome::Delivered => {
                target.set(None);
                error.set(None);
                true
            }
            Outcome::Rejected(message) => {
                error.set(Some(message));
                false
            }
            _ => false,
        }
    }
    fn rail_feedback(&self, mut p: ui::Rail, request: Request, close: bool) -> bool {
        let delivered = match self.rail(request) {
            Outcome::Moved { index, len, group } => {
                p.live
                    .set(crate::ui::sortable::rail_announcement((index, len, group)));
                p.error.set(None);
                true
            }
            Outcome::Delivered | Outcome::Opened => {
                p.error.set(None);
                true
            }
            Outcome::Rejected(error) => {
                p.error.set(Some(error));
                false
            }
            Outcome::NoOp => false,
        };
        if close && delivered {
            p.menu.set(None);
            p.prepared.set(None);
        }
        delivered
    }
}
