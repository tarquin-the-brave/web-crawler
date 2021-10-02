package main

import (
	"fmt"
	"golang.org/x/net/html"
	"net/http"
	"net/url"
	"os"
	"strings"
)

func main() {
	c := make(chan struct {
		From  url.URL
		To    []url.URL
		Error error
	})

	arg := os.Args[1]
	base, err := url.Parse(arg)
	if err != nil {
		fmt.Println(arg, "is not a URL.")
	}
	graph := map[url.URL][]url.URL{}
	toGet := map[url.URL]struct{}{*base: {}}

	for {
		if len(toGet) == 0 {
			break
		}
		freq := 0
		for u := range toGet {
			go getLinks(u, base, c)
			freq += 1
		}

		for i := 0; i < freq; i++ {
			links := <-c
			if links.Error == nil {
				graph[links.From] = links.To
				for _, link := range links.To {
					_, alreadySeen := graph[link]
					if !alreadySeen {
						toGet[link] = struct{}{}
					}
				}
			}
			delete(toGet, links.From)
		}
	}

	totLinks := 0
	for linkFrom, links := range graph {
		totLinks += len(links)
		fmt.Println(linkFrom.String(), "links to:")
		for _, link := range links {
			fmt.Println("\t", link.String())
		}
	}

	fmt.Println("Found", len(graph), "unique pages")
	fmt.Println("Found", totLinks, "total links")
}

func getLinks(u url.URL, base *url.URL, c chan struct {
	From  url.URL
	To    []url.URL
	Error error
}) {
	links, err := getLinksInner(u, base)
	if err == nil {
		fmt.Println("Got", len(links), "links from", u.String())
	} else {
		fmt.Println("Broken link", u)
	}
	c <- struct {
		From  url.URL
		To    []url.URL
		Error error
	}{u, links, err}
}

func getLinksInner(ug url.URL, base *url.URL) ([]url.URL, error) {
	rsp, err := http.Get(ug.String())
	if err != nil {
		fmt.Println(err)
		return []url.URL{}, err
	}
	defer rsp.Body.Close()

	z := html.NewTokenizer(rsp.Body)
	links := []url.URL{}

	for {
		tt := z.Next()

		switch {
		case tt == html.ErrorToken:
			// End of the document, we're done
			return links, nil
		case tt == html.StartTagToken:
			t := z.Token()

			// Check if the token is an <a> tag
			isAnchor := t.Data == "a"
			if !isAnchor {
				continue
			}

			// Extract the href value, if there is one
			ok, u := getHref(t)
			if !ok {
				continue
			}

			link, err := url.Parse(u)
			if err != nil {
				continue
			}

			if !link.IsAbs() {
				link = base.ResolveReference(link)
			}
			// Make sure the url is in the base domain

			if link.Hostname() == base.Hostname() &&
				!strings.HasSuffix(link.String(), ".png") &&
				!strings.HasSuffix(link.String(), ".jpeg") &&
				!strings.HasSuffix(link.String(), ".jpg") &&
				!strings.HasSuffix(link.String(), ".htm") &&
				!strings.HasSuffix(link.String(), ".pdf") {
				links = append(links, *link)
			}
		}
	}

}

// Helper function to pull the href attribute from a Token
func getHref(t html.Token) (ok bool, href string) {
	// Iterate over token attributes until we find an "href"
	for _, a := range t.Attr {
		if a.Key == "href" {
			href = a.Val
			ok = true
		}
	}

	// "bare" return will return the variables (ok, href) as
	// defined in the function definition
	return
}
