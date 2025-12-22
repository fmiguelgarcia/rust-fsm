mod test_helpers;

use nxt_fsm::*;
use test_helpers::state_machine_proc;

mod byte_processor_tests {
	use super::*;
	use buffer_processor::{Output, State, StateMachine};
	use test_case::test_case;

	/// Custom Input type with non-static lifetime for the buffer processor.
	#[derive(Debug)]
	pub enum BufferInput<'a> {
		/// Process a buffer slice with a non-static lifetime
		Process(&'a [u8]),
		/// Flush the processor
		Flush,
	}

	// State machine using custom Input type with non-static lifetime
	state_machine! {
		#[derive(Debug, PartialEq, Eq, Clone)]
		#[allow(unused)]
		#[state_machine(input(BufferInput<'__input_lifetime>))]
		buffer_processor(Idle)

		use super::BufferInput;

		Idle => {
			// NOTE: `data.len() == 4` is not allowed!
			Process(&'a [u8]) if |data: &[u8]| data.len() > 4 => Processing,
			Process(&'a [u8]) if |data: &[u8]| data.len() < 4 => Idle [TooSmall],
		},
		Processing(Flush) => Idle,
		Validating(Flush) => Idle
	}

	#[test_case( [BufferInput::Process(&[])], [Some(Output::TooSmall)], [State::Idle] => Ok(()); "Empty data")]
	#[test_case( [BufferInput::Process(&[1,2,3,4])], [], [] => Err(TransitionImpossibleError); "Data len 4 is invalid transition")]
	#[test_case( [BufferInput::Process(&[1,2,3,4,5])], [None], [State::Processing] => Ok(()); "Process data")]
	#[test_case( [BufferInput::Process(&[1,2,3,4,5]), BufferInput::Flush], [None, None], [State::Processing, State::Idle] => Ok(()); "Process data and flush")]
	fn test<'a, I, O, S>(inputs: I, exp_outputs: O, exp_states: S) -> Result<(), TransitionImpossibleError>
	where
		I: IntoIterator<Item = BufferInput<'a>>,
		O: IntoIterator<Item = Option<Output>>,
		S: IntoIterator<Item = State>,
	{
		let mut machine = StateMachine::new();
		state_machine_proc(&mut machine, inputs, exp_outputs, exp_states)
	}
}
