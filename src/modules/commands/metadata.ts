type Metadata = {
  tag_name: string
  assets: {
    name: string
    browser_download_url: string
  }[]
}

const fetchData = async (): Promise<Metadata> => {
  return await (await fetch('https://api.github.com/repos/SkinsRestorer/SkinsRestorer/releases/latest')).json()
}
let metadata = await fetchData()
setInterval(() => {
  void fetchData().catch(console.error)
}, 1_000 * 60) // 60 requests per hour

export function getMetadata() {
  return metadata
}
