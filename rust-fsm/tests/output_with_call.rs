/// Test for output with call closures
use rust_fsm::*;

use std::ops::Range;
use test_case::test_case;

// Define a custom output type for the calculator
#[derive(Debug, PartialEq)]
pub enum CalcOutput {
    Result(i32),
}

state_machine! {
    #[derive(Debug, PartialEq)]
    #[state_machine(output(CalcOutput))]
    calculator(Idle)

    use super::CalcOutput;

    Idle => {
        Add(i32, i32) => Idle [|a: &i32, b: &i32| CalcOutput::Result(a + b)],
        Multiply(i32, i32) => Idle [|x: &i32, y: &i32| CalcOutput::Result(x * y)],
        Reset => Idle
    }
}

#[test]
fn test_calculator_with_closures() {
    let mut machine = calculator::StateMachine::new();

    // Test addition
    let result = machine.consume(&calculator::Input::Add(5, 3)).unwrap();
    assert_eq!(result, Some(CalcOutput::Result(8)));
    assert_eq!(machine.state(), &calculator::State::Idle);

    // Test multiplication
    let result = machine.consume(&calculator::Input::Multiply(4, 7)).unwrap();
    assert_eq!(result, Some(CalcOutput::Result(28)));
    assert_eq!(machine.state(), &calculator::State::Idle);

    // Test reset (no output)
    let result = machine.consume(&calculator::Input::Reset).unwrap();
    assert!(result.is_none());
}

#[derive(Debug, PartialEq)]
pub enum StringOutput {
    Length(usize),
    Combined(String),
}

state_machine! {
    #[derive(Debug)]
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
    let result = machine
        .consume(&string_processor::Input::Process("hello".to_string()))
        .unwrap();
    assert_eq!(result, Some(StringOutput::Length(5)));

    // Test concatenation
    let result = machine
        .consume(&string_processor::Input::Concat(
            "Hello".to_string(),
            "World".to_string(),
        ))
        .unwrap();
    assert_eq!(
        result,
        Some(StringOutput::Combined("HelloWorld".to_string()))
    );
}

#[derive(Debug, PartialEq)]
pub enum ValidatorOutput {
    Valid,
    Invalid,
}

state_machine! {
    #[derive(Debug)]
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

    machine
        .consume(&validator::Input::CheckRange(value, range.start, range.end))
        .unwrap()
        .unwrap()
}
