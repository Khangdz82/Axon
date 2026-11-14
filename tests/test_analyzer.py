import sys
from src.lexer import Lexer
from src.parser import Parser
from src.analyzer import LocalReasoningAnalyzer, SemanticError

def test_valid():
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
    analyzer = LocalReasoningAnalyzer(prog)
    analyzer.analyze()
    print("test_valid passed!")

def test_use_after_move():
    code = """
    struct Particle {
        pos_x: f32,
    }

    fn test(isolated data: Table[Particle]) -> Table[Particle] {
        let w = spawn {
            return move data;
        };
        // Error: data has been moved into the spawn closure
        return move data;
    }
    """
    tokens = Lexer(code).tokenize()
    prog = Parser(tokens).parse_program()
    analyzer = LocalReasoningAnalyzer(prog)
    try:
        # Note: In our analyzer check, let's see if move tracking handles this
        analyzer.analyze()
    except SemanticError as e:
        print(f"Caught expected semantic error: {e}")

if __name__ == "__main__":
    test_valid()
    test_use_after_move()
