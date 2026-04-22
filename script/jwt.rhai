// Usage: recon --script jwt [SECRET]
//
// Sign a sample JWT, print it, verify.

let secret = if args.len() > 1 { args[1] } else { "demo-secret" };
let claims = #{
    sub: "alice",
    iss: "recon",
    iat: now(),
    custom_field: true,
};
let token = jwt::sign(claims, secret);
print(`token: ${token}`);

let view = jwt::view(token);
print(`payload.sub: ${view.payload.sub}`);

let result = jwt::validate(token, secret);
print(`valid: ${result.valid}`);
return if result.valid { 0 } else { 1 };
