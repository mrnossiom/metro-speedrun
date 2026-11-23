#let station-asset-map = (
  "Cambronne": "cambronne",
  "La Motte-Picquet - Grenelle": "la_motte_picquet",
  "Avenue Émile Zola": "avenue_emile_zola",
  "Invalides": "invalides",
  "Champs-Élysées": "champs_elysees_clemenceau",
  "Concorde": "concorde",
  "Madeleine": "madeleine",
  "Saint-Lazare": "saint_lazare",
  "Havre-Caumartin": "havre_caumartin",
  "Chaussée d'Antin - La Fayette": "chaussee_d_antin",
  "Gare de l'Est": "est",
  "Gare du Nord": "nord",
  "Stalingrad": "stalingrad",
  "Jaurès": "jaures",
  "Place des Fêtes": "place_des_fetes",
  "Porte des Lilas": "porte_des_lilas",
  "Saint-Fargeau": "saint_fargeau",
);

#{
  station-asset-map
    .values()
    .map(name => name + ".png")
    .map(file => asset("stations/" + file, read("stations/" + file, encoding: none)))
    .join()
}

#asset("route/sikora.lss", read("sikora-route.lss", encoding: none))

#show: document.with("route/sikora.pdf")

#title[Route Sikora]

#let station-segment(idx, (line: prev-line), (station,), (line: next-line)) = [
  #pagebreak()

  == Station #station (#prev-line → #next-line)

  #image(
    "transitions/"
      + str(idx)
      + "_"
      + station-asset-map.at(station)
      + "_"
      + lower(prev-line)
      + "-"
      + lower(next-line)
      + ".png",
  )
]

#let line-segment((station: prev-station), (line, direction, position), (station: next-station)) = [
  == Ligne #line vers #direction (#prev-station → #next-station)
  #if position != none { [Position: #position] }
]

#let route(segments) = {
  // allow station windows to see first and last station
  segments.insert(0, (line: "source"))
  segments.push((line: "target"))

  let station-list = segments
    .filter((segment) => "station" in segment)
    .map((station) => station.station)

  let station-segments = segments
    .windows(3)
    .filter(((_, segment, _)) => "station" in segment)
    .enumerate()
    .map(((idx, (args))) => station-segment(idx, ..args))
  let line-segments = segments
    .windows(3)
    .filter(((_, segment, _)) => "line" in segment)
    .map(((args)) => line-segment(..args))

  line-segments.push([])
  list(..station-list) + station-segments.zip(line-segments, exact: true).flatten().join()
}

#route((
  (station: "Cambronne"),
  (line: "M6", direction: "Étoile", position: "Avant"),
  (station: "La Motte-Picquet - Grenelle"),
  (line: "M10", direction: "Boulogne", position: "Avant"),
  (station: "Avenue Émile Zola"),
  (line: "M10", direction: "Gare d'Austerliz", position: "1/3 Avant"),
  (station: "La Motte-Picquet - Grenelle"),
  (line: "M8", direction: "Pointe du Lac", position: "1/3 Avant"),
  (station: "Invalides"),
  (line: "M13", direction: "St-Denis/Courtilles", position: "1/5 Arrière"),
  (station: "Champs-Élysées"),
  (line: "M1", direction: "Vincennes", position: "Arrière"),
  (station: "Concorde"),
  (line: "M12", direction: "Mairie d'Aubervilliers", position: "Avant"),
  (station: "Madeleine"),
  (line: "M14", direction: "Mairie de Saint-Ouen", position: "Avant"),
  (station: "Saint-Lazare"),
  (line: "M3", direction: "Gallieni", position: "Arrière"),
  (station: "Havre-Caumartin"),
  (line: "M9", direction: "Montreuil", position: "Arrière"),
  (station: "Chaussée d'Antin - La Fayette"),
  (line: "M7", direction: "La Courneuve", position: "Arrière"),
  (station: "Gare de l'Est"),
  (line: "M4", direction: "Glignacourt", position: "Arrière"),
  (station: "Gare du Nord"),
  (line: "M5", direction: "Bobigny", position: "Arrière"),
  (station: "Stalingrad"),
  (line: "M2", direction: "Nation", position: "Arrière"),
  (station: "Jaurès"),
  (line: "M7bis", direction: "Pré-Saint-Gervais", position: "Arrière"),
  (station: "Place des Fêtes"),
  (line: "M11", direction: "Mairie des Lilas", position: "1/3 Arrière"),
  (station: "Porte des Lilas"),
  (line: "M3bis", direction: "Gambetta", position: "Arrière"),
  (station: "Saint-Fargeau"),
))
