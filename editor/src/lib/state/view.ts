// Funções do viewport registradas pelo componente Viewport, usadas por comandos e atalhos.
export const viewApi = {
  frameScreen: (_instant?: boolean) => {},
  frameSelection: () => {},
  zoomTo: (_zoom: number) => {},
  center: () => ({ x: 640, y: 360 }),
};
