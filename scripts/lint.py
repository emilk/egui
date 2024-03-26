#!/usr/bin/env python
"""
Runs custom linting on Rust code.
"""

import argparse
import os
import re
import sys


def lint_file_path(filepath, args) -> int:
    with open(filepath) as f:
        lines_in = f.readlines()

    errors, lines_out = lint_lines(filepath, lines_in)

    for error in errors:
        print(error)

    if args.fix and lines_in != lines_out:
        with open(filepath, "w") as f:
            f.writelines(lines_out)
        print(f"{filepath} fixed.")

    return len(errors)


def lint_lines(filepath, lines_in):
    last_line_was_empty = True

    errors = []
    lines_out = []
    prev_line = ""

    for line_nr, line in enumerate(lines_in):
        line_nr = line_nr + 1

        # TODO(emilk): only # and /// on lines before a keyword

        pattern = (
            r"^\s*((///)|((pub(\(\w*\))? )?((impl|fn|struct|enum|union|trait)\b))).*$"
        )
        if re.match(pattern, line):
            stripped = prev_line.strip()
            last_line_was_empty = (
                stripped == ""
                or stripped.startswith("#")
                or stripped.startswith("//")
                or stripped.endswith("{")
                or stripped.endswith("(")
                or stripped.endswith("\\")
                or stripped.endswith('r"')
                or stripped.endswith("]")
            )
            if not last_line_was_empty:
                errors.append(
                    f"{filepath}:{line_nr}: for readability, add newline before `{line.strip()}`"
                )
                lines_out.append("\n")

        if re.search(r"\(mut self.*-> Self", line) and "pub(crate)" not in line:
            if prev_line.strip() != "#[inline]":
                errors.append(
                    f"{filepath}:{line_nr}: builder methods should be marked #[inline]"
                )
                lines_out.append("#[inline]")


        if re.search(r"TODO[^(]", line):
            errors.append(
                f"{filepath}:{line_nr}: write 'TODO(username):' instead"
            )

        if (
            "(target_os" in line
            and filepath.startswith("./crates/egui/")
            and filepath != "./crates/egui/src/os.rs"
        ):
            errors.append(
                f"{filepath}:{line_nr}: Don't use `target_os` - use ctx.os() instead."
            )

        lines_out.append(line)

        prev_line = line

    return errors, lines_out


def test_lint():
    should_pass = [
        "hello world",
        """
        /// docstring
        foo

        /// docstring
        bar
        """,
        """
        #[inline]
        pub fn with_color(mut self, color: Color32) -> Self {
            self.color = color;
            self
        }
        """,
    ]

    should_fail = [
        """
        /// docstring
        foo
        /// docstring
        bar
        """,
        """
        // not inlined
        pub fn with_color(mut self, color: Color32) -> Self {
            self.color = color;
            self
        }
        """,
    ]

    for code in should_pass:
        errors, _ = lint_lines("test.py", code.split("\n"))
        assert len(errors) == 0, f"expected this to pass:\n{code}\ngot: {errors}"

    for code in should_fail:
        errors, _ = lint_lines("test.py", code.split("\n"))
        assert len(errors) > 0, f"expected this to fail:\n{code}"

    pass


def main():
    test_lint()  # Make sure we are bug free before we run!

    parser = argparse.ArgumentParser(description="Lint Rust code with custom linter.")
    parser.add_argument(
        "files",
        metavar="file",
        type=str,
        nargs="*",
        help="File paths. Empty = all files, recursively.",
    )
    parser.add_argument(
        "--fix", dest="fix", action="store_true", help="Automatically fix the files"
    )

    args = parser.parse_args()

    num_errors = 0

    if args.files:
        for filepath in args.files:
            num_errors += lint_file_path(filepath, args)
    else:
        script_dirpath = os.path.dirname(os.path.realpath(__file__))
        root_dirpath = os.path.abspath(f"{script_dirpath}/..")
        os.chdir(root_dirpath)

        exclude = set(["target", "target_ra", "target_wasm"])
        for root, dirs, files in os.walk(".", topdown=True):
            dirs[:] = [d for d in dirs if d not in exclude]
            for filename in files:
                if filename.endswith(".rs"):
                    filepath = os.path.join(root, filename)
                    num_errors += lint_file_path(filepath, args)

    if num_errors == 0:
        print(f"{sys.argv[0]} finished without error")
        sys.exit(0)
    else:
        print(f"{sys.argv[0]} found {num_errors} errors.")
        sys.exit(1)


if __name__ == "__main__":
    main()
