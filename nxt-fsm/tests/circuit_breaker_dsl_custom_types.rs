/// A dummy implementation of the Circuit Breaker pattern to demonstrate
/// capabilities of its library DSL for defining finite state machines.
/// https://martinfowler.com/bliki/CircuitBreaker.html
use nxt_fsm::*;
use std::{
	sync::{Arc, Mutex},
	time::Duration,
};

pub enum Input {
	Successful,
	Unsuccessful,
	TimerTriggered,
}

pub enum State {
	Closed,
	HalfOpen,
	Open,
}

pub enum Output {
	SetupTimer,
}

state_machine! {
	#[state_machine(input(crate::Input), state(crate::State), output(crate::Output))]
	circuit_breaker(Closed)

	Closed(Unsuccessful) => Open [SetupTimer],
	Open(TimerTriggered) => HalfOpen,
	HalfOpen => {
		Successful => Closed,
		Unsuccessful => Open [SetupTimer]
	}
}

#[test]
fn circuit_breaker_dsl_custom_types() {
	let machine = circuit_breaker::StateMachine::new();

	// Unsuccessful request
	let machine = Arc::new(Mutex::new(machine));
	{
		let mut lock = machine.lock().unwrap();
		let res = lock.consume(&Input::Unsuccessful).unwrap();
		assert!(matches!(res, Some(Output::SetupTimer)));
		assert!(matches!(lock.state(), &State::Open));
	}

	// Set up a timer
	let machine_wait = machine.clone();
	std::thread::spawn(move || {
		std::thread::sleep(Duration::from_millis(500));
		let mut lock = machine_wait.lock().unwrap();
		let res = lock.consume(&Input::TimerTriggered).unwrap();
		assert!(res.is_none());
		assert!(matches!(lock.state(), &State::HalfOpen));
	});

	// Try to pass a request when the circuit breaker is still open
	let machine_try = machine.clone();
	std::thread::spawn(move || {
		std::thread::sleep(Duration::from_millis(100));
		let mut lock = machine_try.lock().unwrap();
		let res = lock.consume(&Input::Successful);
		assert!(matches!(res, Err(TransitionImpossibleError)));
		assert!(matches!(lock.state(), &State::Open));
	});

	// Test if the circit breaker was actually closed
	std::thread::sleep(Duration::from_millis(700));
	{
		let mut lock = machine.lock().unwrap();
		let res = lock.consume(&Input::Successful).unwrap();
		assert!(res.is_none());
		assert!(matches!(lock.state(), &State::Closed));
	}
}
