function add(a: int, b: int): int {
    let sum = 0;

    for (let i = 0; i < 10; i = i + 1) {
        if (i == 5) {
            continue;
        }

        sum = sum + i;
    }

    if (sum > 20) {
        return sum;
    } else {
        return 0;
    }
}

function main() {
    add(10, 10);
}
