/// Test for guards with tuple variant inputs
use rust_fsm::*;

state_machine! {
    #[derive(Debug, PartialEq, Eq)]
    payment_system(Idle)

    Idle => {
        StartPayment(u32) if |amount: &u32| *amount >= 100 => Processing,
        StartPayment(u32) if |amount: &u32| *amount < 100 => Idle [InsufficientAmount],
        Cancel => Idle
    },
    Processing => {
        Complete => Success,
        Fail => Failed
    },
    Failed => {
        Retry(u32) if |attempts: &u32| *attempts < 3 => Processing,
        Retry(u32) if |attempts: &u32| *attempts >= 3 => Failed [MaxRetriesExceeded],
        Cancel => Idle
    },
    Success(Reset) => Idle
}

#[test]
fn test_guards_with_amount() {
    let mut machine = payment_system::StateMachine::new();

    // Test with insufficient amount (< 100)
    let res = machine.consume(&payment_system::Input::StartPayment(50));
    // Should stay in Idle state
    assert_eq!(machine.state(), &payment_system::State::Idle);
    // Should have output
    if res != Ok(Some(payment_system::Output::InsufficientAmount)) {
        panic!("Expected InsufficientAmount output");
    }

    // Test with sufficient amount (>= 100)
    let res = machine.consume(&payment_system::Input::StartPayment(150));
    assert!(res.is_ok());
    // Should transition to Processing
    assert_eq!(machine.state(), &payment_system::State::Processing);

    // Complete the payment
    let res = machine.consume(&payment_system::Input::Complete);
    assert!(res.is_ok());
    assert_eq!(machine.state(), &payment_system::State::Success);
}

#[test]
fn test_guards_with_retry_logic() {
    let mut machine = payment_system::StateMachine::new();

    // Start with sufficient payment
    machine
        .consume(&payment_system::Input::StartPayment(200))
        .unwrap();
    assert_eq!(machine.state(), &payment_system::State::Processing);

    // Fail the payment
    machine.consume(&payment_system::Input::Fail).unwrap();
    assert_eq!(machine.state(), &payment_system::State::Failed);

    // Retry with attempts < 3 (should go back to Processing)
    let res = machine.consume(&payment_system::Input::Retry(1));
    assert!(res.is_ok());
    assert_eq!(machine.state(), &payment_system::State::Processing);

    // Fail again
    machine.consume(&payment_system::Input::Fail).unwrap();
    assert_eq!(machine.state(), &payment_system::State::Failed);

    // Retry with attempts >= 3 (should stay in Failed)
    let res = machine.consume(&payment_system::Input::Retry(3));
    assert!(res.is_ok());
    assert_eq!(machine.state(), &payment_system::State::Failed);
    // Should have output
    if res != Ok(Some(payment_system::Output::MaxRetriesExceeded)) {
        panic!("Expected MaxRetriesExceeded output");
    }
}

#[test]
fn test_guards_multiple_fields() {
    state_machine! {
        #[derive(Debug, PartialEq)]
        user_system(Pending)

        Pending => {
            UserData(String, u32, bool) if |_name: &String, age: &u32, premium: &bool| *age >= 18 && *premium => VipUser,
            UserData(String, u32, bool) if |_name: &String, age: &u32, _premium: &bool| *age >= 18 => RegularUser,
            UserData(String, u32, bool) if |_name: &String, age: &u32, _premium: &bool| *age < 18 => MinorUser [ParentalConsentRequired],
        },
        VipUser(Downgrade) => RegularUser,
        RegularUser(Upgrade) => VipUser,
        MinorUser(Approve) => RegularUser
    }

    let mut machine = user_system::StateMachine::new();

    // Test VIP user (age >= 18 and premium)
    let res = machine.consume(&user_system::Input::UserData("Alice".to_string(), 25, true));
    assert!(res.is_ok());
    assert_eq!(machine.state(), &user_system::State::VipUser);

    // Reset
    let mut machine = user_system::StateMachine::new();

    // Test regular user (age >= 18 but not premium)
    let res = machine.consume(&user_system::Input::UserData("Bob".to_string(), 20, false));
    assert!(res.is_ok());
    assert_eq!(machine.state(), &user_system::State::RegularUser);

    // Reset
    let mut machine = user_system::StateMachine::new();

    // Test minor user (age < 18)
    let res = machine.consume(&user_system::Input::UserData(
        "Charlie".to_string(),
        16,
        false,
    ));
    assert!(res.is_ok());
    assert_eq!(machine.state(), &user_system::State::MinorUser);
    if res != Ok(Some(user_system::Output::ParentalConsentRequired)) {
        panic!("Expected ParentalConsentRequired output");
    }
}
