import "dotenv/config";
import { Search } from "@upstash/search";

type KnowledgeContent = {
  text: string;
  section: string;
  title?: string;
};
type KnowledgeDocument = {
  id: string;
  content: KnowledgeContent;
};

const searchUrl = process.env.UPSTASH_SEARCH_REST_URL;
const searchToken = process.env.UPSTASH_SEARCH_REST_TOKEN;

if (searchUrl == null || searchToken == null) {
  throw new Error(
    "Both UPSTASH_SEARCH_REST_URL and UPSTASH_SEARCH_REST_TOKEN must be provided",
  );
}

// Initialize Upstash Search client
const search = new Search({
  url: searchUrl,
  token: searchToken,
});

const index = search.index<KnowledgeContent>("knowledge-base");

async function fetchIndex() {
  const response = await fetch("https://skinsrestorer.net/llms.txt");
  if (!response.ok) {
    throw new Error(`Failed to fetch index: ${response.status}`);
  }
  return await response.text();
}

function parseIndex(content: string) {
  const lines = content.split("\n");
  const pages: { path: string; title: string; section: string }[] = [];
  let currentSection = "";
  for (const line of lines) {
    if (line.startsWith("## ")) {
      currentSection = line.slice(3).trim();
    } else if (line.startsWith("- [")) {
      const match = line.match(/- \[(.*?)\]\((.*?)\): (.*)/);
      if (match?.[1] && match[2]) {
        const title = match[1];
        const path = match[2];
        pages.push({ path, title, section: currentSection });
      }
    }
  }
  return pages;
}

async function fetchPageContent(path: string) {
  const url = `https://skinsrestorer.net${path}.mdx`;
  const response = await fetch(url);
  if (!response.ok) {
    console.warn(`Failed to fetch ${url}: ${response.status}`);
    return null;
  }
  return await response.text();
}

async function setupKnowledgeBase(dryRun: boolean) {
  if (!dryRun) {
    console.log("Resetting index...");
    await index.reset();
    console.log("Index reset.");
  }

  const indexContent = await fetchIndex();
  const pages = parseIndex(indexContent);

  let chunkId = 0;
  const batchSize = 100;
  let batch: KnowledgeDocument[] = [];
  let totalChunks = 0;

  for (const page of pages) {
    const content = await fetchPageContent(page.path);
    if (!content) continue;

    // Split content into meaningful chunks
    const chunks = content
      .split(/\n\s*\n/) // Split by double line breaks (paragraphs)
      .map((chunk) => chunk.trim())
      .filter((chunk) => chunk.length > 50); // Only keep substantial chunks

    for (const chunk of chunks) {
      batch.push({
        id: `chunk-${chunkId++}`,
        content: {
          text: chunk,
          section: page.section,
          title: page.title,
        },
      });
      totalChunks++;

      if (batch.length >= batchSize) {
        if (dryRun) {
          console.log(`Would upsert batch of ${batch.length} chunks:`);
          batch.forEach((item, idx) => {
            console.log(
              `  ${idx + 1}. Title: ${item.content.title}, Section: ${item.content.section}`,
            );
          });
        } else {
          await index.upsert(batch);
          console.log(`Upserted ${batch.length} chunks`);
        }
        batch = [];
      }
    }
  }

  // Handle remaining batch
  if (batch.length > 0) {
    if (dryRun) {
      console.log(`Would upsert final batch of ${batch.length} chunks:`);
      batch.forEach((item, idx) => {
        console.log(
          `  ${idx + 1}. Title: ${item.content.title}, Section: ${item.content.section}`,
        );
      });
    } else {
      await index.upsert(batch);
      console.log(`Upserted ${batch.length} chunks`);
    }
  }

  console.log(`Total chunks processed: ${totalChunks}`);
}

// Check for dry run flag
const dryRun = process.argv.includes("--dry-run");

// Run setup
setupKnowledgeBase(dryRun).catch(console.error);
