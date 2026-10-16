//! Deadzone numeric toolbar rendering and supplied intent callbacks.
use crate::{
    components::{Button, ButtonSize, ButtonVariant, Field, NumberInput},
    instruments::{Event, View},
};
use dioxus::prelude::*;
use inputforge_core::action::Action;
#[component]
pub(crate) fn Toolbar(view: View, send: EventHandler<Event>) -> Element {
    let Action::Deadzone { config } = &view.value else {
        return rsx! {};
    };
    let values = [
        config.low(),
        config.center_low(),
        config.center_high(),
        config.high(),
    ];
    let labels = ["Low", "CL", "CH", "High"];
    let ids = ["dz-low", "dz-cl", "dz-ch", "dz-high"];
    rsx! {
        div{class:"if-deadzone__toolbar",
            for field in 0..4 {
                Field{label:labels[field].to_owned(),for_id:Some(ids[field].to_owned()),error:view.error.clone(),
                    NumberInput{id:Some(ids[field].to_owned()),value:values[field],min:view.bounds[field].0,max:view.bounds[field].1,step:0.01,precision:Some(2),
                        oncommit:move |value|send.call(Event::Number{field,value}),onstep:move |value|send.call(Event::Number{field,value})}
                }
            }
            Button{variant:ButtonVariant::Secondary,size:ButtonSize::Sm,onclick:move |_|send.call(Event::Reset),"Reset"}
        }
    }
}
