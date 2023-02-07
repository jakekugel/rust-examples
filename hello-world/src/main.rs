fn main() {
    let mut terrain = [[0u8; 10]; 10];

    for x = 0..10 {
        for y = 0.. 10 {
            if x == 0 || x == 9 || y == 0 || y == 9 {
                terrain[x][y] = 0;
            } else {
                terrain[x][y] = 1;
            }
        }
    }

    x = 5;
    y = 5;

    for move = 0..100 {
        match terrain[x][y] {
            1 => println!("Dark woods");
            2 => println!("Impassable mountains");
            _ => println!("Unknown terrain");
        }

        io::stdout().flush().unwrap();

        let mut command = String::new();

        io::stdin()
            .read_line(&mut command)
            .expect("Failed to read line");

        command = command.trim().to_string().to_lowercase();

        if command == "N" {
            y = y + 1;
        } else if command == "S" {
            y = y - 1;
        } else if command == "E" {
            x = x + 1;
        } else if command == "W" {
            x = x - 1;
        }
    }
}
