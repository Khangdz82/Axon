from src.lexer import Lexer
from src.parser import Parser

code = """
struct Particle {
    pos_x: f32,
    pos_y: f32,
    vel_x: f32,
    vel_y: f32,
    mass: f32,
}

fn integrate_physics(mut particles: Table[Particle], dt: f32) {
    for p in particles {
        p.pos_x += p.vel_x * dt;
        p.pos_y += p.vel_y * dt;
    }
}

fn simulate_world(isolated world_data: Table[Particle]) -> Table[Particle] {
    let worker = spawn {
        integrate_physics(mut world_data, 0.016);
        return move world_data;
    };

    return worker.join();
}
"""

tokens = Lexer(code).tokenize()
prog = Parser(tokens).parse_program()
print(f"Parsed successfully: {len(prog.structs)} structs, {len(prog.functions)} functions")
for fn in prog.functions:
    param_str = ", ".join(f"{p.name}: {p.intent} {p.type_node.name}" for p in fn.params)
    print(f"  Fn: {fn.name}({param_str}), body stmts: {len(fn.body)}")
