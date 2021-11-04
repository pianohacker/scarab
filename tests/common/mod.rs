use scarab::compiler::compile;
use scarab::parser::parse_implicit_form_list;
use scarab::vm::Vm;

pub fn exec(code: &str) -> String {
    let program = parse_implicit_form_list(code.chars()).expect("parsing failed");

    let instructions = compile(program).expect("compilation failed");

    let mut debug_output = Vec::new();
    {
        let mut vm = Vm::new(&mut debug_output);
        vm.load(instructions);
        vm.run().expect("running program failed");
    }

    String::from_utf8(debug_output).unwrap().trim().to_string()
}
