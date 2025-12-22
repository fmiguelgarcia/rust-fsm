/// Test for tuple variant input support in the state_machine! macro
use nxt_fsm::*;

state_machine! {
	turnstile(Locked)

	Locked => {
		Coin(u32) => Unlocked,
		Push => Locked
	},
	Unlocked(Push) => Locked
}

state_machine! {
	#[allow(unused)]
	complex_machine(Start)

	Start => {
		Data(String, u32, bool) => Processing,
		Skip => End
	},
	Processing => {
		Complete => End,
		Retry(u32) => Processing
	},
	End(Reset) => Start
}

#[test]
fn tuple_variant_input() {
	let mut machine = turnstile::StateMachine::new();

	// Initial state should be Locked
	assert!(matches!(machine.state(), &turnstile::State::Locked));

	// Insert coin (tuple variant with u32 value)
	let res = machine.consume(&turnstile::Input::Coin(100));
	assert!(res.is_ok());
	assert!(matches!(machine.state(), &turnstile::State::Unlocked));

	// Push through (unit variant)
	let res = machine.consume(&turnstile::Input::Push);
	assert!(res.is_ok());
	assert!(matches!(machine.state(), &turnstile::State::Locked));

	// Try to push when locked
	let res = machine.consume(&turnstile::Input::Push);
	assert!(res.is_ok());
	assert!(matches!(machine.state(), &turnstile::State::Locked));

	// Insert different coin amount
	let res = machine.consume(&turnstile::Input::Coin(50));
	assert!(res.is_ok());
	assert!(matches!(machine.state(), &turnstile::State::Unlocked));
}

#[test]
fn tuple_variant_pattern_matching() {
	// Test that we can pattern match on tuple variants
	let coin_input = turnstile::Input::Coin(100);

	match coin_input {
		turnstile::Input::Coin(amount) => {
			assert_eq!(amount, 100);
		},
		_ => panic!("Expected Coin variant"),
	}
}

#[test]
fn complex_tuple_variants() {
	let mut machine = complex_machine::StateMachine::new();

	// Test multi-field tuple variant
	let res = machine.consume(&complex_machine::Input::Data("test".to_string(), 42, true));
	assert!(res.is_ok());
	assert!(matches!(machine.state(), &complex_machine::State::Processing));

	// Test single-field tuple variant
	let res = machine.consume(&complex_machine::Input::Retry(3));
	assert!(res.is_ok());
	assert!(matches!(machine.state(), &complex_machine::State::Processing));

	// Test unit variant
	let res = machine.consume(&complex_machine::Input::Complete);
	assert!(res.is_ok());
	assert!(matches!(machine.state(), &complex_machine::State::End));

	// Test reset
	let res = machine.consume(&complex_machine::Input::Reset);
	assert!(res.is_ok());
	assert!(matches!(machine.state(), &complex_machine::State::Start));
}
