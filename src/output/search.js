const items = [...INPUT_ITEMS];

const urlSearchParams = new URLSearchParams(window.location.search);
const query = urlSearchParams.get('query') || "";

const searchQuery = document.getElementById("search_query");
searchQuery.value = query;

const TOO_COMMON = ["the", "be", "to", "of", "and", "a", "in", "that", "have", "i", "it", "for"];

function splitWords(s) {
    return s.split(/\W+/).filter(w => w.length > 0 && !TOO_COMMON.includes(w)).map(w => w.toLowerCase());
}

function makeDistinct(a) {
    return [...new Set(a)];
}

const queryWords = makeDistinct(splitWords(query));
let maxDistinctMatches = 0;

for (const item of items) {
    let itemWords = [];

    let eat = s => {
        if (typeof s !== "string") {
            return;
        }
        itemWords = itemWords.concat(splitWords(s));
    }

    eat(item.name);
    if (Array.isArray(item.categories)) {
        for (const category of item.categories) {
            eat(category);
        }
    }
    eat(item.description);
    eat(item.pageTextContent);
    eat(item.month);
    eat(item.year);

    item.matches = 0;
    item.distinctMatches = [];

    for (const itemWord of itemWords) {
        for (const queryWord of queryWords) {
            if (itemWord.startsWith(queryWord)) {
                item.matches += 1;
                item.distinctMatches.push(queryWord);
            }
        }
    }

    item.distinctMatches = makeDistinct(item.distinctMatches);
    maxDistinctMatches = Math.max(maxDistinctMatches, item.distinctMatches.length);
}

const filteredItems = items.filter(i => i.matches > 0 && i.distinctMatches.length == maxDistinctMatches);
filteredItems.sort((a, b) => {
    if (b.distinctMatches.length == a.distinctMatches.length) {
        return b.matches - a.matches;
    } else {
        return b.distinctMatches.length - a.distinctMatches.length;
    }
});

const MAX = 25;

let resultCount = filteredItems.length;
let truncated = false;
if (filteredItems.length > MAX) {
    filteredItems.length = MAX;
    truncated = true;
}

const searchResultsContainer = document.getElementById("page_main_body_search_results");


while (searchResultsContainer.hasChildNodes()) {
    searchResultsContainer.removeChild(searchResultsContainer.lastChild);
}

for (const result of filteredItems) {
    const searchResultContainer = document.createElement("a");
    searchResultContainer.href = result.path;
    searchResultContainer.classList.add("thumbnail_container");

    const thumbnailElement = document.createElement("img");
    thumbnailElement.src = result.thumbnailPath;
    thumbnailElement.title = result.name;
    thumbnailElement.alt = result.description || result.name;
    thumbnailElement.classList.add("thumbnail");
    searchResultContainer.appendChild(thumbnailElement);

    searchResultsContainer.appendChild(searchResultContainer);
}

if (resultCount == 0) {
    let metaRobots = Array.from(document.getElementsByTagName("meta")).filter(e => e.name == 'robots')[0];

    if (!metaRobots) {
        metaRobots = document.createElement('meta');
        metaRobots.name = 'robots';
        document.head.appendChild(metaRobots);
    }

    metaRobots.content = ["noindex"].concat(
        metaRobots.content.split(",").filter(d => d != "index")
    ).join(",");
}

const pageMainBody = document.getElementById("page_main_body");
const summary = document.createElement("p");
summary.innerText = `Found ${resultCount} ${resultCount == 1 ? "result" : "results"}${truncated ? ", showing top " + MAX : ""}.`
pageMainBody.appendChild(summary);