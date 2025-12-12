/// Test for match guards with multiple bindings in tuple variants
use crate::coin_machine::{Input, Output, State, StateMachine};

use rust_fsm::*;
use test_case::test_case;

state_machine! {
    #[derive(Debug, PartialEq, Clone)]
    coin_machine(Locked)

    Locked => {
        Coin(u32, String) match (cash, description) {
            (0..25, _) => Locked [RefundInsufficient],
            (25.., _) => Unlocked [DoublePayment],
            _ => Unlocked
        },
        Push => Locked [AccessDenied]
    },
    Unlocked => {
        Push => Locked,
        Coin(u32, String) => Unlocked [AlreadyUnlocked]
    }
}

#[test_case([Input::Coin(10, "Cash".to_string())], [Some(Output::RefundInsufficient)], [State::Locked]; "Insufficient" )]
#[test_case([Input::Coin(25, "Credit Card".to_string())], [Some(Output::DoublePayment)], [State::Unlocked]; "DoublePayment" )]
#[test_case([Input::Push, Input::Coin(25, "Crypto".to_string())], [Some(Output::AccessDenied), Some(Output::DoublePayment)], [State::Locked, State::Unlocked]; "Push & DoublePayment")]
fn multiple_bindings<I, O, S>(inputs: I, exp_outputs: O, exp_states: S)
where
    I: IntoIterator<Item = Input>,
    O: IntoIterator<Item = Option<Output>>,
    S: IntoIterator<Item = State>,
{
    let mut coin_machine = StateMachine::new();

    let (outputs, states): (Vec<_>, Vec<_>) = inputs
        .into_iter()
        .map(|input| {
            let output = coin_machine.consume(&input).unwrap();
            let state = coin_machine.state().clone();
            (output, state)
        })
        .unzip();

    assert_eq!(outputs, exp_outputs.into_iter().collect::<Vec<_>>());
    assert_eq!(states, exp_states.into_iter().collect::<Vec<_>>());
}
