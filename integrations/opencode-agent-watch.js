const hook = "/path/to/tmux-agent-watch/scripts/opencode-hook.sh"

export const TmuxAgentWatch = async ({ $ }) => {
  const publish = async (state, message = "") => {
    await $`${hook} ${state} ${message}`.quiet()
  }

  return {
    event: async ({ event }) => {
      const properties = event.properties ?? {}

      if (event.type === "permission.asked") {
        await publish("permission", properties.permission?.title ?? "Approval required")
      } else if (event.type === "session.idle") {
        await publish("idle")
      } else if (event.type === "session.error") {
        await publish("error", properties.error?.message ?? "Agent stopped with an error")
      } else if (event.type === "session.status") {
        const status = properties.status?.type
        if (status === "busy" || status === "retry") await publish("working")
      }
    },
  }
}
