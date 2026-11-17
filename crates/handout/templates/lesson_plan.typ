// Printable class schedule (Phase W8): the schedule as a table, then one
// page per class in the plan-of-class format. All text comes from
// /data.json, so nothing in it is read as Typst markup.

#let data = json("/data.json")
#let labels = data.labels
#let ink = luma(20)
#let muted = luma(100)
#let rule = luma(190)

#set document(title: data.title)
#set text(font: "Libertinus Serif", size: 10.5pt, lang: lower(data.lang.split("-").at(0)), fill: ink)
#set par(justify: true, leading: 0.62em, spacing: 0.9em)

#set page(
  paper: "a4",
  margin: (x: 1.8cm, top: 2.2cm, bottom: 2cm),
  header: context {
    if here().page() > 1 {
      set text(size: 8.5pt, fill: muted)
      data.title
      h(1fr)
      data.subtitle
      v(-6pt)
      line(length: 100%, stroke: 0.4pt + rule)
    }
  },
  footer: context {
    set text(size: 8.5pt, fill: muted)
    h(1fr)
    counter(page).display("1")
  },
)

#let heading-label(body) = text(size: 9pt, tracking: 0.12em, fill: muted, upper(body))

#let list-of(items) = {
  for item in items {
    block(above: 3pt, below: 3pt, grid(columns: (10pt, 1fr), [•], item))
  }
}

#let activities(items) = {
  for item in items {
    block(above: 3pt, below: 3pt, grid(
      columns: (1fr, auto),
      column-gutter: 8pt,
      [#text(weight: "bold", item.kind) — #item.text],
      text(fill: muted, item.minutes),
    ))
  }
}

// The schedule.
#heading-label(labels.schedule)
#v(2pt)
#text(size: 17pt, weight: "bold", data.title)
#v(-4pt)
#text(fill: muted, data.subtitle)
#v(8pt)
#table(
  columns: (auto, auto, auto, 1fr, auto),
  stroke: (x, y) => if y == 0 { (bottom: 0.6pt + ink) } else { (bottom: 0.3pt + rule) },
  inset: (x: 4pt, y: 5pt),
  align: (left, left, left, left, left),
  table.header(
    text(weight: "bold", labels.date),
    text(weight: "bold", labels.time),
    text(weight: "bold", labels.kind),
    text(weight: "bold", labels.topics),
    text(weight: "bold", labels.status),
  ),
  ..data.sessions.map(s => (
    s.weekday + " " + s.date,
    s.time,
    s.kind,
    s.topics.join(" · "),
    s.status,
  )).flatten(),
)

// One page per class.
#for (i, s) in data.sessions.enumerate() {
  pagebreak()
  heading-label(labels.lesson_plan + " " + str(i + 1))
  v(2pt)
  text(size: 15pt, weight: "bold", s.weekday + " " + s.date + " · " + s.time)
  v(-4pt)
  text(fill: muted, s.kind + " · " + s.minutes + " · " + s.status)
  if s.topics.len() > 0 {
    v(6pt)
    heading-label(labels.topics)
    list-of(s.topics)
  }
  if s.objectives.len() > 0 {
    v(6pt)
    heading-label(labels.objectives)
    list-of(s.objectives)
  }
  if s.activities.len() > 0 {
    v(6pt)
    heading-label(labels.activities)
    activities(s.activities)
  }
  if s.materials.len() > 0 {
    v(6pt)
    heading-label(labels.materials)
    list-of(s.materials)
  }
  if s.homework.len() > 0 {
    v(6pt)
    heading-label(labels.homework)
    activities(s.homework)
  }
  if s.link != "" {
    v(6pt)
    heading-label(labels.link)
    block(text(size: 9.5pt, s.link))
  }
  if s.note != "" {
    v(6pt)
    heading-label(labels.note)
    block(fill: luma(240), inset: 8pt, width: 100%, s.note)
  }
  if s.objectives.len() == 0 and s.activities.len() == 0 {
    v(6pt)
    text(fill: muted, labels.empty)
  }
}
