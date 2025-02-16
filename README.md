# web-crawler

**UPDATE:** This exercise has been extended with a [Part
2](#part-2---policing-the-wikipedia-game) that follows on from the main
exercise.  For a greater challenge, go straight to part 2.

> Write a web crawler that can be given a URL and show all the URLs that URL
  links to within the domain.  It will then find all the URLs that those URLs
  link to, and carry on until it has built the graph of the website with the
  links between pages. Display this graph to the user.

E.g. if `www.example.com` linked to two pages: `www.example.com/foo` &
`www.example.com/bar`.  And `www.example.com/foo` linked to
`www.example.com/bar` you might see an output like:

```
www.example.com links to:
  www.example.com/foo
  www.example.com/bar
www.example.com/foo links to:
  www.example.com/bar

Found 3 links to 2 unique URLs.
```

anything linked outside of the `www.example.com` domain wouldn't be visited or
included in this output.

The will be a number of design decisions to make which might effect what's
included in your graph of links between URLs.  Stating how many links you find
will help comparison of completeness and performance of different solutions you
might try.

# Adding a Solution

Submit a PR to add a binary under `src/bin/`.

## Things to Consider

- speed - how to make it as fast as possible?
- correctness/completeness
  + does it collect all the links it can?
  + does it handle relative links?
  + what to do when links are broken?
  + do you include links to sub-domains?
  + what to do about fragments & queries?
  + etc...
- input URL - does this have to be the base URL?
- output - how to display the results? terminal? serve to HTML to view in
  browser?
- progress - how to show the user it is working?

If you want to focus on _speed_ and the design of how to achieve it: some
basic, by no means correct or complete, implementations of some of "the other
bits" are included in `src/lib.rs`.  You can use these and or improve them.

## Testing Performance

The solution should ultimately work on any website, and it's worth testing
against a range of websites.  It would be interesting to see how the
performance is impacted between smaller and bigger sites. Starting with smaller
websites that do a decent amount of linking internally is probably best.

To test:

```
cargo build --release
time cargo run --release --bin <your solution binary> -- <url>
```

Some `<url>` suggestions:

```
https://python-poetry.org
https://serde.rs
https://www.beyondmeat.com
```

# Part 2 - Wikiracing

Adapt your program (or write a new one if it's starting here) so instead of 
building a graph of internal links in a website you instead make it perform
[Wikiracing](https://en.wikipedia.org/wiki/Wikiracing)!

In Wikiracing you agree the Wikipedia articles you want to start and finish at,
and compete to see who can get from the start to the finish first, following 
only internal links.
