import sys
import unittest
from pathlib import Path

# Set up PYTHONPATH
root_dir = Path(__file__).resolve().parent.parent
sys.path.insert(0, str(root_dir))

from src.lexer import Lexer
from src.parser import Parser
from src.analyzer import LocalReasoningAnalyzer, SemanticError
from src.codegen_c import CCodeGen

class TestAetherCompiler(unittest.TestCase):
    def test_lexer(self):
        code = "struct Particle { pos_x: f32 } fn run(mut p: Table[Particle]) {}"
        tokens = Lexer(code).tokenize()
        types = [t.type for t in tokens if t.type != "EOF"]
        self.assertIn("STRUCT", types)
        self.assertIn("TABLE", types)
        self.assertIn("MUT", types)

    def test_local_reasoning_success(self):
        code = """
        struct Particle { pos_x: f32, vel_x: f32 }
        fn sim(mut p: Table[Particle], dt: f32) {
            for x in p {
                x.pos_x += x.vel_x * dt;
            }
        }
        """
        prog = Parser(Lexer(code).tokenize()).parse_program()
        analyzer = LocalReasoningAnalyzer(prog)
        analyzer.analyze()  # Should not raise

    def test_second_class_reference_rejection(self):
        # Structs cannot contain references
        code = """
        struct Node {
            ref: &Node
        }
        """
        prog = Parser(Lexer(code).tokenize()).parse_program()
        analyzer = LocalReasoningAnalyzer(prog)
        with self.assertRaises(SemanticError):
            analyzer.analyze()

    def test_linear_isolation_rejection(self):
        # Using a variable after moving it into a concurrent spawn block
        code = """
        struct Particle { pos_x: f32 }
        fn bad(isolated data: Table[Particle]) -> Table[Particle] {
            let w = spawn {
                return move data;
            };
            return move data; // VIOLATION: use after move
        }
        """
        prog = Parser(Lexer(code).tokenize()).parse_program()
        analyzer = LocalReasoningAnalyzer(prog)
        with self.assertRaises(SemanticError):
            analyzer.analyze()

    def test_codegen_soa_output(self):
        code = """
        struct Particle { pos_x: f32, vel_x: f32 }
        fn sim(mut p: Table[Particle], dt: f32) {
            for x in p {
                x.pos_x += x.vel_x * dt;
            }
        }
        """
        prog = Parser(Lexer(code).tokenize()).parse_program()
        cg = CCodeGen(prog)
        c_code = cg.generate()
        self.assertIn("typedef struct Aether_Table_Particle", c_code)
        self.assertIn("float* AETHER_RESTRICT pos_x;", c_code)
        self.assertIn("float* AETHER_RESTRICT vel_x;", c_code)
        self.assertIn("AETHER_SIMD_LOOP", c_code)

if __name__ == "__main__":
    unittest.main()
