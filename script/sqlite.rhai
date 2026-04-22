// Usage: recon --script sqlite
//
// In-memory SQLite round-trip: CREATE, INSERT, SELECT.

let db = sqlite(":memory:");
db.exec("CREATE TABLE people (id INTEGER PRIMARY KEY, name TEXT, age INTEGER)");
db.exec("INSERT INTO people (name, age) VALUES (?, ?)", ["alice", 30]);
db.exec("INSERT INTO people (name, age) VALUES (?, ?)", ["bob", 25]);

let all = db.query("SELECT * FROM people ORDER BY age");
print(`${all.len()} people:`);
for row in all {
    print(`  ${row.id}: ${row.name} (${row.age})`);
}

let count = db.query_value("SELECT COUNT(*) FROM people", []);
assert(count == 2, "count matches");
return 0;
