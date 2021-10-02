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
	arg := os.Args[1]
	base, err := url.Parse(arg)
	if err != nil {
		fmt.Println(arg, "is not a URL.")
	}

	cLinks := make(chan struct {
		From  url.URL
		To    []url.URL
		Error error
	})
	cUrl := make(chan url.URL)
	cGraph := make(chan map[url.URL][]url.URL)

	go fetcher(base, cUrl, cLinks)
	go graphBuilder(cLinks, cUrl, cGraph)
	cUrl <- *base

	graph := <-cGraph

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

func fetcher(base *url.URL, cUrl chan url.URL, cLinks chan struct {
	From  url.URL
	To    []url.URL
	Error error
}) {
	for {
		u := <-cUrl
		go getLinks(u, base, cLinks)
	}
}

func graphBuilder(cLinks chan struct {
	From  url.URL
	To    []url.URL
	Error error
}, cUrl chan url.URL, cGraph chan map[url.URL][]url.URL) {
	graph := map[url.URL][]url.URL{}
	inFlight := map[url.URL]struct{}{}

	for {
		links := <-cLinks
		delete(inFlight, links.From)
		if links.Error == nil {
			graph[links.From] = links.To
			for _, link := range links.To {
				linkStr := link.String()
				if strings.HasSuffix(linkStr, ".png") ||
					strings.HasSuffix(linkStr, ".jpeg") ||
					strings.HasSuffix(linkStr, ".jpg") ||
					strings.HasSuffix(linkStr, ".htm") ||
					strings.HasSuffix(linkStr, ".pdf") {
					continue
				}
				link.RawQuery = ""
				link.Fragment = ""
				if _, found := graph[link]; found {
					continue
				}

				if _, found := inFlight[link]; found {
					continue
				}
				cUrl <- link
				inFlight[link] = struct{}{}
			}
		}

		if len(inFlight) == 0 {
			break
		}
	}

	cGraph <- graph
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
			if link.Hostname() == base.Hostname() {
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
