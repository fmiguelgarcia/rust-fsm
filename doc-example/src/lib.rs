use nxt_fsm::state_machine;

state_machine! {
	/// A dummy implementation of the Circuit Breaker pattern to demonstrate
	/// capabilities of its library DSL for defining finite state machines.
	/// https://martinfowler.com/bliki/CircuitBreaker.html
	pub circuit_breaker(Closed)

	Closed(Unsuccessful) => Open [SetupTimer],
	Open(TimerTriggered) => HalfOpen,
	HalfOpen => {
		Successful => Closed,
		Unsuccessful => Open [SetupTimer]
	}
}

// Define a custom output type for the calculator
#[derive(Debug, PartialEq)]
pub enum CalcOutput {
	Result(i32),
	Clear,
}

state_machine! {
	#[derive(Debug, PartialEq)]
	#[state_machine(output(CalcOutput))]
	/// A simple calculator state machine demonstrating arithmetic operations and error handling.
	/// It uses Inputs carrying data, like _operands_, and closures to generate output.
	pub calculator(Idle)

	use super::CalcOutput;

	Idle => {
		Add(i32, i32) => Idle [|a: &i32, b: &i32| CalcOutput::Result(a + b)],
		Multiply(i32, i32) => Idle [|x: &i32, y: &i32| CalcOutput::Result(x * y)],
		Divide(i32, i32) match (x, y) {
			(_, 0) => ErrDivByZero,
			(__arg0, __arg1) => Idle [ |x, y| CalcOutput::Result(x/y)]
		}
	},
	ErrDivByZero(Reset) => Idle [Clear]
}
