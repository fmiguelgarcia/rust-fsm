/// Test for guards with reference types in tuple variant inputs.
///
/// This test verifies that guards can correctly use reference parameters
/// when the input tuple variant contains reference types like `&'static str`.
use rust_fsm::*;

// Test with &'static str reference type in input
state_machine! {
    #[derive(Debug, PartialEq, Eq)]
    #[allow(unused)]
    string_parser(Idle)

    Idle => {
        Parse(&'static str) match text {
            "start" => Running,
            "stop" => Stopped,
            _ => Idle [UnknownCommand]
        }
    },
    Running => {
        Parse(&'static str) if |text: &str| text.starts_with("cmd:") => Running [CommandReceived],
        Parse(&'static str) if |text: &str| text == "stop" => Stopped
    },
    Stopped(Reset) => Idle
}

#[test]
fn test_str_reference_with_match_guard() {
    let mut machine = string_parser::StateMachine::new();

    // Test "start" command
    let res = machine.consume(&string_parser::Input::Parse("start"));
    assert!(res.is_ok());
    assert_eq!(machine.state(), &string_parser::State::Running);

    // Reset for next test
    let mut machine = string_parser::StateMachine::new();

    // Test "stop" command
    let res = machine.consume(&string_parser::Input::Parse("stop"));
    assert!(res.is_ok());
    assert_eq!(machine.state(), &string_parser::State::Stopped);

    // Reset for next test
    let mut machine = string_parser::StateMachine::new();

    // Test unknown command
    let res = machine.consume(&string_parser::Input::Parse("unknown"));
    assert!(res.is_ok());
    assert_eq!(machine.state(), &string_parser::State::Idle);
    assert_eq!(res, Ok(Some(string_parser::Output::UnknownCommand)));
}

#[test]
fn test_str_reference_with_closure_guard() {
    let mut machine = string_parser::StateMachine::new();

    // Start the machine
    machine
        .consume(&string_parser::Input::Parse("start"))
        .unwrap();
    assert_eq!(machine.state(), &string_parser::State::Running);

    // Test command prefix matching
    let res = machine.consume(&string_parser::Input::Parse("cmd:hello"));
    assert!(res.is_ok());
    assert_eq!(machine.state(), &string_parser::State::Running);
    assert_eq!(res, Ok(Some(string_parser::Output::CommandReceived)));

    // Test stop command
    let res = machine.consume(&string_parser::Input::Parse("stop"));
    assert!(res.is_ok());
    assert_eq!(machine.state(), &string_parser::State::Stopped);
}

// Test with static slice reference type
state_machine! {
    #[derive(Debug, PartialEq, Eq)]
    #[allow(unused)]
    byte_processor(Ready)

    Ready => {
        Process(&'static [u8]) if |data: &[u8]| !data.is_empty() => Processing,
        Process(&'static [u8]) if |data: &[u8]| data.is_empty() => Ready [EmptyData]
    },
    Processing(Done) => Ready
}

#[test]
fn test_slice_reference_guard() {
    let mut machine = byte_processor::StateMachine::new();

    // Test with non-empty data
    let res = machine.consume(&byte_processor::Input::Process(&[1, 2, 3]));
    assert!(res.is_ok());
    assert_eq!(machine.state(), &byte_processor::State::Processing);

    // Complete processing
    machine.consume(&byte_processor::Input::Done).unwrap();
    assert_eq!(machine.state(), &byte_processor::State::Ready);

    // Test with empty data
    let res = machine.consume(&byte_processor::Input::Process(&[]));
    assert!(res.is_ok());
    assert_eq!(machine.state(), &byte_processor::State::Ready);
    assert_eq!(res, Ok(Some(byte_processor::Output::EmptyData)));
}

// Test mixing reference and non-reference types
state_machine! {
    #[derive(Debug, PartialEq, Eq)]
    #[allow(unused)]
    mixed_types(Init)

    Init => {
        Data(&'static str, u32) match (name, value) {
            ("admin", 0..) => AdminMode,
            (_, 100..) => HighValue,
            (_, _) => LowValue
        }
    },
    AdminMode(Exit) => Init,
    HighValue(Exit) => Init,
    LowValue(Exit) => Init
}

#[test]
fn test_mixed_reference_and_value_types() {
    let mut machine = mixed_types::StateMachine::new();

    // Test admin mode
    let res = machine.consume(&mixed_types::Input::Data("admin", 50));
    assert!(res.is_ok());
    assert_eq!(machine.state(), &mixed_types::State::AdminMode);

    // Reset
    let mut machine = mixed_types::StateMachine::new();

    // Test high value (non-admin)
    let res = machine.consume(&mixed_types::Input::Data("user", 200));
    assert!(res.is_ok());
    assert_eq!(machine.state(), &mixed_types::State::HighValue);

    // Reset
    let mut machine = mixed_types::StateMachine::new();

    // Test low value
    let res = machine.consume(&mixed_types::Input::Data("guest", 50));
    assert!(res.is_ok());
    assert_eq!(machine.state(), &mixed_types::State::LowValue);
}

// Test output generation with reference types
state_machine! {
    #[derive(Debug, PartialEq)]
    #[allow(unused)]
    #[state_machine(output(EchoOutput))]
    echo_machine(Listening)

    use super::EchoOutput;

    Listening => {
        Message(&'static str) => Listening [|msg: &str| EchoOutput::Echo(msg.len())]
    }
}

#[derive(Debug, PartialEq)]
pub enum EchoOutput {
    Echo(usize),
}

#[test]
fn test_output_with_reference_type() {
    let mut machine = echo_machine::StateMachine::new();

    // Test message echo with length
    let res = machine.consume(&echo_machine::Input::Message("hello"));
    assert!(res.is_ok());
    assert_eq!(machine.state(), &echo_machine::State::Listening);
    assert_eq!(res, Ok(Some(EchoOutput::Echo(5))));

    // Test with longer message
    let res = machine.consume(&echo_machine::Input::Message("hello world"));
    assert!(res.is_ok());
    assert_eq!(res, Ok(Some(EchoOutput::Echo(11))));
}

// Test with closure guard that modifies logic based on reference content
state_machine! {
    #[derive(Debug, PartialEq, Eq)]
    #[allow(unused)]
    command_processor(Idle)

    Idle => {
        Command(&'static str, u32) if |cmd: &str, priority: &u32| cmd == "urgent" && *priority > 5 => UrgentProcessing,
        Command(&'static str, u32) if |cmd: &str, _: &u32| cmd == "normal" => NormalProcessing,
        Command(&'static str, u32) => Idle [InvalidCommand]
    },
    UrgentProcessing(Complete) => Idle,
    NormalProcessing(Complete) => Idle
}

#[test]
fn test_multiple_ref_params_in_closure_guard() {
    let mut machine = command_processor::StateMachine::new();

    // Test urgent high priority
    let res = machine.consume(&command_processor::Input::Command("urgent", 10));
    assert!(res.is_ok());
    assert_eq!(machine.state(), &command_processor::State::UrgentProcessing);

    // Reset
    machine
        .consume(&command_processor::Input::Complete)
        .unwrap();

    // Test urgent low priority (should fail guard, try next)
    let res = machine.consume(&command_processor::Input::Command("urgent", 3));
    assert!(res.is_ok());
    // Falls through to invalid since no "normal" match
    assert_eq!(machine.state(), &command_processor::State::Idle);
    assert_eq!(res, Ok(Some(command_processor::Output::InvalidCommand)));

    // Test normal command
    let res = machine.consume(&command_processor::Input::Command("normal", 1));
    assert!(res.is_ok());
    assert_eq!(machine.state(), &command_processor::State::NormalProcessing);
}

// =============================================================================
// NON-STATIC REFERENCES with GATs
// =============================================================================
//
// The `StateMachineImpl` trait now uses Generic Associated Types (GATs) for the
// Input type, which allows using non-static references in inputs.
//
// To use non-static references with the DSL:
// 1. Define your own Input type with a lifetime parameter
// 2. Use `#[state_machine(input(YourType<'__input_lifetime>))]`
//
// The special lifetime `'__input_lifetime` is used internally by the macro.

/// Custom Input type with non-static lifetime for the buffer processor.
#[derive(Debug)]
pub enum BufferInput<'a> {
    /// Process a buffer slice with a non-static lifetime
    Process(&'a [u8]),
    /// Validate buffer content
    Validate(&'a str),
    /// Flush the processor
    Flush,
}

// State machine using custom Input type with non-static lifetime
state_machine! {
    #[derive(Debug, PartialEq, Eq)]
    #[allow(unused)]
    #[state_machine(input(BufferInput<'__input_lifetime>))]
    buffer_processor(Idle)

    use super::BufferInput;

    Idle => {
        Process(&'a [u8]) if |data: &[u8]| data.len() >= 4 => Processing,
        Process(&'a [u8]) if |data: &[u8]| data.len() < 4 => Idle [BufferTooSmall],
        Validate(&'a str) if |s: &str| !s.is_empty() => Validating
    },
    Processing(Flush) => Idle,
    Validating(Flush) => Idle
}

#[test]
fn test_non_static_reference_with_guard() {
    let mut machine = buffer_processor::StateMachine::new();

    // Create a local buffer (non-static lifetime!)
    let local_buffer: Vec<u8> = vec![1, 2, 3, 4, 5];

    // Process with sufficient data - this works because GATs allow non-'static refs
    let res = machine.consume(&BufferInput::Process(&local_buffer));
    assert!(res.is_ok());
    assert_eq!(machine.state(), &buffer_processor::State::Processing);

    // Flush and reset
    machine.consume(&BufferInput::Flush).unwrap();
    assert_eq!(machine.state(), &buffer_processor::State::Idle);

    // Process with insufficient data
    let small_buffer: [u8; 2] = [1, 2];
    let res = machine.consume(&BufferInput::Process(&small_buffer));
    assert!(res.is_ok());
    assert_eq!(machine.state(), &buffer_processor::State::Idle);
    assert_eq!(res, Ok(Some(buffer_processor::Output::BufferTooSmall)));
}

#[test]
fn test_non_static_str_reference() {
    let mut machine = buffer_processor::StateMachine::new();

    // Create a local string (non-static lifetime!)
    let local_string = String::from("hello world");

    // Validate with non-empty string - works with non-'static reference
    let res = machine.consume(&BufferInput::Validate(&local_string));
    assert!(res.is_ok());
    assert_eq!(machine.state(), &buffer_processor::State::Validating);
}

/// Another example: Protocol parser with non-static references
#[derive(Debug)]
pub enum ProtocolInput<'a> {
    Header(&'a [u8]),
    Body(&'a [u8]),
    Complete,
}

state_machine! {
    #[derive(Debug, PartialEq, Eq)]
    #[allow(unused)]
    #[state_machine(input(ProtocolInput<'__input_lifetime>))]
    protocol_parser(WaitingHeader)

    use super::ProtocolInput;

    WaitingHeader => {
        Header(&'a [u8]) if |h: &[u8]| h.len() >= 4 && h[0] == 0xAA => WaitingBody,
        Header(&'a [u8]) => WaitingHeader [InvalidHeader]
    },
    WaitingBody => {
        Body(&'a [u8]) if |b: &[u8]| !b.is_empty() => Complete,
        Body(&'a [u8]) => WaitingBody [EmptyBody]
    },
    Complete(Complete) => WaitingHeader
}

#[test]
fn test_protocol_parser_with_non_static_refs() {
    let mut machine = protocol_parser::StateMachine::new();

    // Create local buffers (non-static)
    let header = vec![0xAA, 0x01, 0x02, 0x03];
    let body = vec![0x10, 0x20, 0x30];

    // Parse header
    let res = machine.consume(&ProtocolInput::Header(&header));
    assert!(res.is_ok());
    assert_eq!(machine.state(), &protocol_parser::State::WaitingBody);

    // Parse body
    let res = machine.consume(&ProtocolInput::Body(&body));
    assert!(res.is_ok());
    assert_eq!(machine.state(), &protocol_parser::State::Complete);

    // Complete and reset
    machine.consume(&ProtocolInput::Complete).unwrap();
    assert_eq!(machine.state(), &protocol_parser::State::WaitingHeader);
}

#[test]
fn test_protocol_parser_invalid_header() {
    let mut machine = protocol_parser::StateMachine::new();

    // Invalid header (wrong magic byte)
    let bad_header = vec![0xBB, 0x01, 0x02, 0x03];
    let res = machine.consume(&ProtocolInput::Header(&bad_header));
    assert!(res.is_ok());
    assert_eq!(machine.state(), &protocol_parser::State::WaitingHeader);
    assert_eq!(res, Ok(Some(protocol_parser::Output::InvalidHeader)));

    // Too short header
    let short_header = vec![0xAA, 0x01];
    let res = machine.consume(&ProtocolInput::Header(&short_header));
    assert!(res.is_ok());
    assert_eq!(machine.state(), &protocol_parser::State::WaitingHeader);
    assert_eq!(res, Ok(Some(protocol_parser::Output::InvalidHeader)));
}

// =============================================================================
// Additional test: Complex reference patterns with 'static
// =============================================================================

// Test with multiple static reference types and complex guard logic
state_machine! {
    #[derive(Debug, PartialEq, Eq)]
    #[allow(unused)]
    config_parser(Initial)

    Initial => {
        // Multiple reference fields with match guards
        Configure(&'static str, &'static [u8]) match (key, data) {
            ("debug", [1, ..]) => DebugMode,
            ("debug", _) => Initial [InvalidDebugConfig],
            (_, _) => Initial [UnknownConfig]
        },
        // Use closure guard for more complex conditions
        ConfigureProd(&'static [u8]) if |data: &[u8]| data.len() >= 4 => ProductionMode,
        ConfigureProd(&'static [u8]) => Initial [ProdConfigTooSmall]
    },
    DebugMode(Reset) => Initial,
    ProductionMode(Reset) => Initial
}

#[test]
fn test_multiple_static_references_in_match() {
    let mut machine = config_parser::StateMachine::new();

    // Test debug mode with valid config
    let res = machine.consume(&config_parser::Input::Configure("debug", &[1, 2, 3]));
    assert!(res.is_ok());
    assert_eq!(machine.state(), &config_parser::State::DebugMode);

    // Reset
    machine.consume(&config_parser::Input::Reset).unwrap();

    // Test debug mode with invalid config (first byte not 1)
    let res = machine.consume(&config_parser::Input::Configure("debug", &[0, 2, 3]));
    assert!(res.is_ok());
    assert_eq!(machine.state(), &config_parser::State::Initial);
    assert_eq!(res, Ok(Some(config_parser::Output::InvalidDebugConfig)));

    // Test production mode with sufficient data using closure guard
    let res = machine.consume(&config_parser::Input::ConfigureProd(&[1, 2, 3, 4]));
    assert!(res.is_ok());
    assert_eq!(machine.state(), &config_parser::State::ProductionMode);

    // Reset
    machine.consume(&config_parser::Input::Reset).unwrap();

    // Test production mode with insufficient data
    let res = machine.consume(&config_parser::Input::ConfigureProd(&[1, 2]));
    assert!(res.is_ok());
    assert_eq!(machine.state(), &config_parser::State::Initial);
    assert_eq!(res, Ok(Some(config_parser::Output::ProdConfigTooSmall)));

    // Test unknown key
    let res = machine.consume(&config_parser::Input::Configure("unknown", &[]));
    assert!(res.is_ok());
    assert_eq!(machine.state(), &config_parser::State::Initial);
    assert_eq!(res, Ok(Some(config_parser::Output::UnknownConfig)));
}

// Test closure guards with static string references doing complex matching
state_machine! {
    #[derive(Debug, PartialEq, Eq)]
    #[allow(unused)]
    protocol_handler(Disconnected)

    Disconnected => {
        Connect(&'static str, u16) if |host: &str, port: &u16| {
            host.starts_with("secure.") && *port == 443
        } => SecureConnected,
        Connect(&'static str, u16) if |host: &str, port: &u16| {
            !host.is_empty() && *port > 0
        } => Connected,
        Connect(&'static str, u16) => Disconnected [InvalidConnection]
    },
    Connected(Disconnect) => Disconnected,
    SecureConnected(Disconnect) => Disconnected
}

#[test]
fn test_closure_guards_with_static_str_and_value() {
    let mut machine = protocol_handler::StateMachine::new();

    // Test secure connection
    let res = machine.consume(&protocol_handler::Input::Connect("secure.example.com", 443));
    assert!(res.is_ok());
    assert_eq!(machine.state(), &protocol_handler::State::SecureConnected);

    // Reset
    machine
        .consume(&protocol_handler::Input::Disconnect)
        .unwrap();

    // Test regular connection
    let res = machine.consume(&protocol_handler::Input::Connect("example.com", 8080));
    assert!(res.is_ok());
    assert_eq!(machine.state(), &protocol_handler::State::Connected);

    // Reset
    machine
        .consume(&protocol_handler::Input::Disconnect)
        .unwrap();

    // Test invalid connection (empty host)
    let res = machine.consume(&protocol_handler::Input::Connect("", 80));
    assert!(res.is_ok());
    assert_eq!(machine.state(), &protocol_handler::State::Disconnected);
    assert_eq!(res, Ok(Some(protocol_handler::Output::InvalidConnection)));

    // Test invalid connection (port 0)
    let res = machine.consume(&protocol_handler::Input::Connect("example.com", 0));
    assert!(res.is_ok());
    assert_eq!(machine.state(), &protocol_handler::State::Disconnected);
    assert_eq!(res, Ok(Some(protocol_handler::Output::InvalidConnection)));
}
