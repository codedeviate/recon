// gh.rhai — drive the GitHub gh CLI from a script.
// Usage: recon --script gh.rhai
//
// Requires `gh` on PATH and `gh auth status` to succeed. Skips when
// not configured.

let h = gh();
let auth = #{};
try {
    auth = h.auth_status();
} catch (e) {
    print(`gh not configured (${e}); skipping demo`);
    return;
}

print(`gh active account: ${auth.account}`);
print(`     host:         ${auth.host}`);
print(`     scopes:       ${auth.scopes}`);

// Read-only operations against the current repo.
try {
    let prs = h.pr_list(#{ state: "open", limit: 3 });
    print(`${prs.len()} open PRs:`);
    for pr in prs {
        print(`  #${pr.number} ${pr.title} (${pr.author.login})`);
    }
} catch (e) {
    print(`pr_list failed: ${e}`);
}

try {
    let repo = h.repo_view();
    let owner_login = repo.owner.login ?? "?";
    let default_branch = repo.defaultBranchRef.name ?? "?";
    print(`repo: ${owner_login}/${repo.name} on default branch ${default_branch}`);
} catch (e) {
    print(`repo_view failed: ${e}`);
}

()
