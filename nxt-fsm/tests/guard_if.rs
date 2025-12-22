mod test_helpers;

use nxt_fsm::*;
use payment_system::{
	Input::{self, Complete, Fail, Retry, StartPayment},
	Output::{self, InsufficientAmount},
	State, StateMachine,
};
use test_case::test_case;
use test_helpers::state_machine_proc;

state_machine! {
			/// Test for guards with tuple variant inputs
	#[derive(Clone, Debug, PartialEq, Eq)]
	#[allow(unused)]
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
		Retry(u32) match attempts {
			0..3 => Processing,
			3.. => Failed [MaxRetriesExceeded]
		},
		Cancel => Idle
	},
	Success(Reset) => Idle
}

#[rustfmt::skip]
#[test_case( 
	[StartPayment(50), StartPayment(150), Complete], 
	[Some(InsufficientAmount), None, None], 
	[State::Idle, State::Processing, State::Success] 
	=> Ok(()); "with amount")]
#[test_case( 
	[StartPayment(200), Fail, Retry(1), Fail, Retry(3) ], 
	[None, None, None, None, Some(Output::MaxRetriesExceeded)], 
	[State::Processing, State::Failed, State::Processing, State::Failed, State::Failed]
	=> Ok(()); "with retry logic")]
fn test_guards_payment_system<I, O, S>( inputs: I, exp_outputs: O, exp_states: S) -> Result<(), TransitionImpossibleError>
	where
		I: IntoIterator<Item = Input>,
		O: IntoIterator<Item = Option<Output>>,
		S: IntoIterator<Item = State>,
{
	let mut machine = StateMachine::new();
	state_machine_proc(&mut machine, inputs, exp_outputs, exp_states)
}
