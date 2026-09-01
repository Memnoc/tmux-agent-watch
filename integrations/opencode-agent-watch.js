const hook = "/path/to/tmux-agent-watch/scripts/opencode-hook.sh"

export const TmuxAgentWatch = async ({ $ }) => {
  const publish = async (state) => {
    await $`${hook} ${state}`.quiet()
  }

  return {
    event: async ({ event }) => {
      const properties = event.properties ?? {}

      if (event.type === "permission.asked") {
        await publish("permission")
      } else if (event.type === "session.idle") {
        await publish("idle")
      } else if (event.type === "session.error") {
        await publish("error")
      } else if (event.type === "session.status") {
        const status = properties.status?.type
        if (status === "busy" || status === "retry") await publish("working")
      }
    },
  }
}
