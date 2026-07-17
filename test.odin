package main

@export
test_add :: proc "c" (a, b: i32) -> i32 {
    return a + b
}
