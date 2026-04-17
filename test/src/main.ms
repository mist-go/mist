export struct MyType {
    x: int,
}

export function test(): MyType { }

function add() {
    let output = test().x;
}
