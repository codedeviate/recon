// git.rhai — drive the git CLI from a script.
// Usage: recon --script git.rhai
//
// The demo creates a fresh temp repo, makes a few commits, and
// exercises every git wrapper method against it. No global git
// state is modified.

let tmp = `/tmp/recon-git-demo-${now_ms()}`;
shell(`mkdir -p ${tmp} && cd ${tmp} && git init -q -b master && git config user.email demo@example.com && git config user.name Demo && git commit --allow-empty -q -m initial`);

let g = git(tmp);

print(`branch:   ${g.branch().current}`);
print(`clean:    ${g.is_clean()}`);
print(`HEAD SHA: ${g.rev_parse("HEAD").sub_string(0, 7)}`);

// Make a change and observe status / diff.
shell(`echo 'hello' > ${tmp}/greeting.txt`);
let s = g.status();
print(`untracked: ${s.untracked}`);

g.add("greeting.txt");
print(`staged after add: ${g.status().staged.len()}`);

let c = g.commit("add greeting");
print(`new commit ${c.short_hash}: ${c.subject}`);

// Log.
let entries = g.log(5);
print(`${entries.len()} commits so far`);
for e in entries {
    print(`  ${e.short_hash} ${e.subject}`);
}

// Add a second commit + diff_stat against HEAD~1.
shell(`echo 'world' >> ${tmp}/greeting.txt`);
g.add("greeting.txt");
g.commit("expand greeting");
let stat = g.diff_stat("HEAD~1");
print(`diff_stat: ${stat.files} file, ${stat.insertions} insertions, ${stat.deletions} deletions`);

// Escape hatch.
let v = g.run_text("--version");
print(`version: ${v.trim()}`);

// Clean up.
shell(`rm -rf ${tmp}`);
()
