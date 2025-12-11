/// Simple example of guards with tuple variants
use rust_fsm::*;

state_machine! {
    #[derive(Debug, PartialEq)]
    turnstile(Locked)

    Locked => {
        Coin(u32) if |amount: &u32| *amount >= 50 => Unlocked,
        Coin(u32) if |amount: &u32| *amount < 50 => Locked [RefundInsufficient],
        Push => Locked [AccessDenied]
    },
    Unlocked => {
        Push => Locked,
        Coin(u32) => Unlocked [AlreadyUnlocked]
    }
}

#[test]
fn test_simple_guard() {
    let mut machine = turnstile::StateMachine::new();

    // Insufficient coin
    let res = machine.consume(&turnstile::Input::Coin(25));
    assert_eq!(res, Ok(Some(turnstile::Output::RefundInsufficient)));
    assert_eq!(machine.state(), &turnstile::State::Locked);

    // Sufficient coin
    let res = machine.consume(&turnstile::Input::Coin(50));
    assert_eq!(machine.state(), &turnstile::State::Unlocked);
    assert_eq!(res, Ok(None));

    // Push through
    let res = machine.consume(&turnstile::Input::Push);
    assert!(res.is_ok());
    assert_eq!(machine.state(), &turnstile::State::Locked);
}
