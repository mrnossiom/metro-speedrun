// categories: array<(name: str, key: str, content: str)>
#let tabs(categories) = {
  let keys = categories.map(((name, key, content)) => key)
  for key in keys {
    assert(key.match(regex("([a-zA-Z0-9-])+")) != none, message: "expect simple key, got " + key)
  }

  let radios = categories
    .enumerate()
    .map(((i, (name, key, content))) => html.input(
      type: "radio",
      name: "tabs-category",
      id: "tab-" + key,
      checked: i == 0, // check the first one by default
    ))
  let labels = categories.map(((name, key, content)) => html.elem(
    "label",
    attrs: ("for": "tab-" + key),
    name,
  ))
  let sections = categories.map(((name, key, content)) => html.div(id: "tab-content-" + key, content))

  let style = (
    (
      keys.map(key => "#tab-" + key),
      keys.map(key => "#tab-content-" + key),
    )
      .flatten()
      .join(",")
      + "{ display: none; }",
    keys.map(key => "#tab-" + key + ":checked~#tab-content-" + key).join(",") + "{ display: block; }",
    keys.map(key => "#tab-" + key + ":checked~.labels [for=\"tab-" + key + "\"]").join(",")
      + "{ color: var(--color-text-accent); }",
  ).join()

  html.div(
    (
      html.style(style),
      ..radios,
      html.div(class: "labels", labels.join()),
      ..sections,
    ).join(),
  )
}
