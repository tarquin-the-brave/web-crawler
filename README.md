# web-crawler

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
```

anything linked outside of the `www.example.com` domain wouldn't be visited or
included in this output.

# Adding a Solution

Submit a PR to add a binary under `src/bin/`.

## Things to Consider

- speed - how to make it as fast as possible?
- correctness/completeness
  + does it collect all the links it can?
  + does it handle relative links?
  + what to do when links are broken?
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

Some suggestions:

```
https://python-poetry.org
https://serde.rs
https://beyondmeat.com
```
