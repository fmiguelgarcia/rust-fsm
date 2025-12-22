use crate::traffic_light::{
	Input::{self, Timer, Velocity},
	Output, State, StateMachine,
};

use nxt_fsm::*;
use std::sync::atomic::{AtomicU32, Ordering::Relaxed};
use test_case::test_case;

static YELLOW_PASS: AtomicU32 = AtomicU32::new(0);

fn log_before_transition(state: &State, input: &Input) {
	println!("Before transition, from state {state:?} with input {input:?}")
}

fn count_yellow_pass(_pre_state: &State, _input: &Input, state: &State, output: Option<&Output>) {
	if state == &State::Yellow && output == Some(&Output::Pass) {
		YELLOW_PASS.fetch_add(1, Relaxed);
	}
}

state_machine! {
	#[derive(Debug, PartialEq)]
	#[state_machine(before_transition(crate::log_before_transition), after_transition(crate::count_yellow_pass))]
	traffic_light(Red)

	Red(Timer) => Green [Go],
	Green => {
			Timer => Yellow,
			Velocity(i32) if |velocity: &i32| *velocity < 30 => Yellow [ Break ],
			Velocity(i32) if |velocity: &i32| *velocity >= 30 => Yellow [ Pass ],
		},
	Yellow(Timer) => Red [Stop],
}

#[test_case([], 0; "No events" )]
#[test_case([Timer, Timer, Timer], 0; "Only timers")]
#[test_case([Timer, Velocity(30) ], 1; "One Pass")]
#[test_case([Timer, Velocity(50), Timer, Timer, Velocity(80) ], 2; "Two Pass")]
fn before_after_tx<I>(events: I, exp_yellow_pass: u32)
where
	I: IntoIterator<Item = Input>,
{
	let mut machine = StateMachine::new();

	let curr_yellow_pass = YELLOW_PASS.load(Relaxed);
	for e in events.into_iter() {
		let _ = machine.consume(&e);
	}

	let diff_yellow_pass = YELLOW_PASS.load(Relaxed) - curr_yellow_pass;
	assert_eq!(diff_yellow_pass, exp_yellow_pass);
}
