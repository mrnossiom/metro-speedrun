set RealStations;
set RealLines;
set RealArcs dimen 3;

set Stations := RealStations union {"source", "target"};
set Lines := RealLines union {"dummy"};
set Arcs :=
	RealArcs
	union ({"source"} cross RealStations cross {"dummy"})
	union (RealStations cross {"target"} cross {"dummy"})
	;
var follow{Arcs} binary;

var flow{Arcs} >= 0;
var y{Stations} >= 0;

s.t. OddDegree{v in RealStations}:
	sum{(u,v,l) in Arcs} follow[u,v,l] = sum{(v,w,l) in Arcs} follow[v,w,l];
s.t. SourceExpectDegree:
	sum{("source",v,l) in Arcs} follow["source",v,l] = 1;
s.t. TargetExpectDegree:
	sum{(u,"target",l) in Arcs} follow[u,"target",l] = 1;

s.t. VisitLineAtLeastOnce{l in Lines}:
	sum{(u,v,l) in Arcs} follow[u,v,l] >= 1;

s.t. FlowCapacity{(u,v,l) in Arcs}:
	card(Stations) * follow[u,v,l] >= flow[u,v,l];
s.t. FlowLinearity{v in Stations diff {"source"}}:
	sum{(u,v,l) in Arcs} flow[u,v,l] - sum{(v,w,l) in Arcs} flow[v,w,l] >= y[v];
s.t. FlowConnectivity{v in Stations}:
	y[v] - sum{(u,v,l) in Arcs} follow[u,v,l] - sum{(v,w,l) in Arcs} follow[v,w,l] >= 0;

minimize NbTransitions:
	# remove dummy transition
	sum{(u, v, l) in Arcs} follow[u,v,l] - 2;

solve;

for {(u,v,l) in Arcs : follow[u,v,l] && l in RealLines}
{
	printf "source,target,line\n";
	printf "%s,%s,%s\n", u, v, l;
}

end;

