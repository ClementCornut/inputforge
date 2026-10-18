//! Curve toolbar rendering and supplied intent callbacks.
use crate::{
    components::{Button, ButtonSize, ButtonVariant, Switch, TabItem, Tabs},
    instruments::{Event, View},
};
use dioxus::prelude::*;
use inputforge_core::{action::Action, processing::curves::ResponseCurve};
#[component]
pub(crate) fn Toolbar(view: View, send: EventHandler<Event>) -> Element {
    let Action::ResponseCurve { curve } = &view.value else {
        return rsx! {};
    };
    let (kind, symmetric) = match curve {
        ResponseCurve::PiecewiseLinear { symmetric, .. } => ("linear", *symmetric),
        ResponseCurve::CubicSpline { symmetric, .. } => ("spline", *symmetric),
        ResponseCurve::CubicBezier { symmetric, .. } => ("bezier", *symmetric),
    };
    rsx! {
        div{class:"if-curve__toolbar",
            Tabs{value:kind.to_owned(),items:vec![
                TabItem{id:"linear".into(),label:"Linear".into(),controls:None,running:false},
                TabItem{id:"spline".into(),label:"Spline".into(),controls:None,running:false},
                TabItem{id:"bezier".into(),label:"Bezier".into(),controls:None,running:false},
            ],onchange:move |id|send.call(Event::Type(id))}
            Switch{checked:symmetric,onchange:move |evt:FormEvent|send.call(Event::Symmetric(evt.checked())),label:Some("Symmetric".to_owned())}
            Button{variant:ButtonVariant::Secondary,size:ButtonSize::Sm,onclick:move |_|send.call(Event::Reset),"Reset"}
        }
    }
}
