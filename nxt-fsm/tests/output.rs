mod test_helpers;

use nxt_fsm::*;
use test_helpers::state_machine_proc;

mod calculator_tests {
	use super::*;
	use calculator::{
		Input::{self, Add, Divide, Multiply},
		State, StateMachine,
	};
	use test_case::test_case;

	// Define a custom output type for the calculator
	#[derive(Debug, PartialEq)]
	pub enum COutput {
		Result(i32),
		ErrDiv0,
	}

	state_machine! {
		#[derive(Clone, Debug, PartialEq)]
		#[state_machine(output(COutput))]
		#[allow(unused)]
		calculator(Idle)

		use super::COutput;

		Idle => {
			Add(i32, i32) => Idle [|a: &i32, b: &i32| COutput::Result(a + b)],
			Multiply(i32, i32) => Idle [|x: &i32, y: &i32| COutput::Result(x * y)],
			Divide(i32, i32) match (a,b) {
				(_, 0) => ErrS [ ErrDiv0 ],
				(__arg0, __arg1) => Idle [ |x: &i32, y: &i32| COutput::Result(x/y)]
			},
		},
		ErrS (Reset) => Idle,
	}

	#[test_case([Add(5,3)], [Some(COutput::Result(8))], [State::Idle] ; "Add")]
	#[test_case([Multiply(4,7)], [Some(COutput::Result(28))], [State::Idle] ; "Multiply")]
	#[test_case([Divide(10,2)], [Some(COutput::Result(5))], [State::Idle]  ; "Divide")]
	#[test_case([Divide(10,0)], [Some(COutput::ErrDiv0)], [State::ErrS]  ; "Divide by zero")]
	fn tests<I, O, S>(inputs: I, exp_outputs: O, exp_states: S)
	where
		I: IntoIterator<Item = Input>,
		O: IntoIterator<Item = Option<COutput>>,
		S: IntoIterator<Item = State>,
	{
		let mut machine = StateMachine::new();
		state_machine_proc(&mut machine, inputs, exp_outputs, exp_states).unwrap();
	}
}

mod string_processor_tests {
	use super::*;

	#[derive(Debug, PartialEq)]
	pub enum StringOutput {
		Length(usize),
		Combined(String),
	}

	state_machine! {
		#[derive(Debug)]
		#[allow(unused)]
		#[state_machine(output(StringOutput))]
		string_processor(Ready)

		use super::StringOutput;

		Ready => {
			Process(String) => Ready [|s: &String| StringOutput::Length(s.len())],
			Concat(String, String) => Ready [ |a: &str, b: &str| StringOutput::Combined(format!("{a}{b}"))],
			Clear => Ready
		}
	}

	#[test]
	fn test_string_processor_with_closures() {
		let mut machine = string_processor::StateMachine::new();

		// Test string length
		let result = machine.consume(&string_processor::Input::Process("hello".to_string())).unwrap();
		assert_eq!(result, Some(StringOutput::Length(5)));

		// Test concatenation
		let result =
			machine.consume(&string_processor::Input::Concat("Hello".to_string(), "World".to_string())).unwrap();
		assert_eq!(result, Some(StringOutput::Combined("HelloWorld".to_string())));
	}
}

mod validator_tests {
	use super::*;
	use std::ops::Range;
	use test_case::test_case;

	#[derive(Debug, PartialEq)]
	pub enum ValidatorOutput {
		Valid,
		Invalid,
	}

	state_machine! {
		#[derive(Debug)]
		#[allow(unused)]
		#[state_machine(output(ValidatorOutput))]
		validator(Waiting)

		use super::ValidatorOutput;

		Waiting => {
			CheckRange(i32, i32, i32) => Waiting [|value: &i32, start: &i32, end: &i32| {
				if value >= start && value < end {
					ValidatorOutput::Valid
				} else {
					ValidatorOutput::Invalid
				}
			}],
			Reset => Waiting
		}
	}

	#[test_case( 50, 0..100 => ValidatorOutput::Valid)]
	#[test_case( 150, 0..100 => ValidatorOutput::Invalid)]
	#[test_case( -10, 0..100 => ValidatorOutput::Invalid)]
	#[test_case( 0, 0..100 => ValidatorOutput::Valid)]
	#[test_case( 99, 0..100 => ValidatorOutput::Valid)]
	#[test_case( 100, 0..100 => ValidatorOutput::Invalid)]
	fn test_closure_captures_complex_logic(value: i32, range: Range<i32>) -> ValidatorOutput {
		let mut machine = validator::StateMachine::new();

		machine.consume(&validator::Input::CheckRange(value, range.start, range.end)).unwrap().unwrap()
	}
}
