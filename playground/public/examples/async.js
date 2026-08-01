async function loadUser(id) {
  const response = await Promise.resolve({ id, name: "Ada" });
  return response;
}

loadUser(42).then((user) => {
  console.log("loaded", user.name);
  user;
});
