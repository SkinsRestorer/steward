let metadata: { name?: string } = {}

const fetchData = async (): Promise<void> => {
  try {
    metadata = { ...await (await fetch('https://api.spiget.org/v2/resources/2124/versions/latest')).json() }
  } catch (e) {
    console.error(e)
  }
}

await fetchData()
setInterval(fetchData, 1_000 * 60) // 60 requests per hour

export function getMetadata () {
  return metadata
}
