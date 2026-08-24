console.log(Object.keys(ipc))
export const availableCommands = ipc.commands()
export function invoke(command, payload) {
  return ipc.invoke(command, payload)
}
