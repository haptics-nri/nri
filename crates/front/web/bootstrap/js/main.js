var ACTIONS = [];
var RUNNING = false;
function next() {
    RUNNING = false;

    var after = ACTIONS.shift();
    if (after !== undefined) {
        handle_rpc(after);
    }
}

function handle_rpc(data) {
    if (RUNNING) {
        ACTIONS.push(data);
    } else {
        RUNNING = true;
        words = data.split(' ');
        switch (words[0]) {
            case "msg":
                alert(data.slice(data.indexOf(" ")),
                        null,
                        function () {
                            $("#abort").click();
                        });
                break;
            case "prompt":
                prompt(data.slice(data.indexOf(" ")),
                        function (s) {
                            send("RPC" + window.wsid + " " + s);
                        },
                        function () {
                            send("RPC" + window.wsid + " ABORT");
                        });
                break;
        }
    }
}

function alert(text, thenok, thencancel) {
    thenok = thenok || function () { console.log("alert modal dismissed by clicking OK"); };
    thencancel = thencancel || function () { console.log("alert modal cancelled"); };

    $("#alert .modal-body").html(text);
    $("#alert #ok").unbind('click.alert').bind('click.alert', thenok);
    $("#alert #cancel").unbind('click.alert').bind('click.alert', thencancel);
    $("#alert").modal("show");
}
function confirm(text, thenyes, thenno, thencancel) {
    thenyes = thenyes || function () { console.log("confirm modal dismissed by clicking Yes"); };
    thenno = thenno || function () { console.log("confirm modal dismissed by clicking No"); };
    thencancel = thencancel || function () { console.log("confirm modal cancelled"); };

    $("#confirm .modal-body").html(text);
    $("#confirm #yes").unbind('click.confirm').bind('click.confirm', thenyes);
    $("#confirm #no").unbind('click.confirm').bind('click.confirm', thenno);
    $("#confirm #cancel").unbind('click.confirm').bind('click.confirm', thencancel);
    $("#confirm").modal("show");
}
function prompt(text, thenok, thencancel) {
    thenok = thenok || function (s) { console.log("prompt modal dismissed by clicking OK, contents \"" + s + "\""); };
    thencancel = thencancel || function () { console.log("prompt modal cancelled"); };

    $("#prompt #text").html(text);
    $("#prompt #input").val("");
    if (text.endsWith("scale)")) {
        $("#prompt #input").attr("type", "number");
    } else {
        $("#prompt #input").attr("type", "text");
    }
    $("#prompt #ok").unbind('click.prompt').bind('click.prompt', function () { thenok($("#prompt #input").val()); });
    $("#prompt #cancel").unbind('click.prompt').bind('click.prompt', thencancel);
    $("#prompt").modal({ show: true, backdrop: "static" });
}

function send(s) {
    console.log("send " + s);
    window.socket.send(s);
}

var timer = null;
var dead_man_timer = null;

function dead_man_switch() {
    clearTimeout(dead_man_timer);
    dead_man_timer = setTimeout(dead_man_alarm, 1000);
}

function dead_man_alarm() {
    if (document.response.document.body.innerText =="") {
        alert("Server not responding!");
    }
}

function start_timer() {
    dead_man_switch();

    $("#timer").html("0m0s");
    clearInterval(timer);
    timer = setInterval(update_timer, 1000);
}

function clear_timer() {
    dead_man_switch();

    clearInterval(timer);
    $("#timer").html("");
}

function update_timer() {
    var cur = $("#timer").html();
    var parts = cur.match(/(\d+)m(\d+)s/);
    var min = parseInt(parts[1]);
    var sec = parseInt(parts[2]);

    sec++;
    if (sec >= 60) {
        sec = 0;
        min++;
    }

    $("#timer").html(min + "m" + sec + "s");
}

function set_datadir() {
    prompt("Set data directory",
            function (s) {
                send("DATA" + window.wsid + " set DATADIR " + escape(s));
            });
}

var SCHEDULE = [];
function schedule(f) {
    if (f === undefined) {
        var f = SCHEDULE.shift();
        if (f !== undefined) {
            f();
        }
    } else {
        SCHEDULE.push(f);
    }
}

var DEMO = false;
var FRAME_TIMINGS = {};
var DRAW_TIMINGS = {};

function start_demo() {
    console.log("STARTING DEMO");

    // start cameras
    schedule(function() { $("#start_teensy").click(); });
    schedule(function() { $("#start_bluefox").click(); });
    schedule(function() { $("#start_structure").click(); });

    // get frames
    DEMO = true;
    FRAME_TIMINGS = {'bluefox': [], 'structure': [], 'teensy': []};
    DRAW_TIMINGS = {'bluefox': [], 'structure': [], 'teensy': []};
    SENSOR_DATA = {};
    function bkick() {
        $("#kick_bluefox").click();
        if (DEMO) {
            schedule(skick);
        }
    }
    function skick() {
        $("#kick_structure").click();
        if (DEMO) {
            schedule(tkick);
        }
    }
    function tkick() {
        $("#kick_teensy").click();
        if (DEMO) {
            schedule(bkick);
        }
    }
    schedule(bkick);

    schedule();
}

function stop_demo() {
    console.log("STOPPING DEMO");

    if (DEMO) {
        DEMO = false;

        var fps_real = {};
        var fps_show = {};
        for (cam in FRAME_TIMINGS) {
            var num_diffs = 0;
            var time_diffs = 0;
            var last = null;
            for (frame in FRAME_TIMINGS[cam]) {
                if (frame > 0) {
                    num_diffs += FRAME_TIMINGS[cam][frame].num - last.num;
                    time_diffs += FRAME_TIMINGS[cam][frame].time - last.time;
                }
                last = FRAME_TIMINGS[cam][frame];
            }
            fps_real[cam] = num_diffs / time_diffs * 1000;
            fps_show[cam] = FRAME_TIMINGS[cam].length / time_diffs * 1000;
        }
        console.log({'Real FPS': fps_real, 'Shown FPS': fps_show});
    }
}

window.onload = function() {
    $("#alert").on("hidden.bs.modal", next);
    $("#confirm").on("hidden.bs.modal", next);
    $("#prompt").on("hidden.bs.modal", next);
    $("#prompt").on("keypress", function (e) {
        if (e.which == 13) {
            $("#prompt #ok").click();
            e.preventDefault();
        }
    });
};

function ab2str(buf) {
  return String.fromCharCode.apply(null, new Uint16Array(buf));
}
function str2ab(str) {
  var buf = new ArrayBuffer(str.length*2); // 2 bytes for each char
  var bufView = new Uint16Array(buf);
  for (var i=0, strLen=str.length; i < strLen; i++) {
    bufView[i] = str.charCodeAt(i);
  }
  return buf;
}

SENSOR_DATA = {};

window.socket.onmessage = function (event) {
    console.log(event.data.slice(0, 50).replace(/\n+/g, '') + ' (' + event.data.length + ')');
    words = event.data.split(' ');
    switch (words[0]) {
        case "hello":
            window.wsid = words[1];
            $(".wsid").each(function () { this.value = words[1]; });
            break;
        case "msg":
        case "prompt":
            handle_rpc(event.data);
            break;
        case "kick":
            var sensor = words[1];
            var framenum = words[2];
            var payload = words[3];

            $("." + sensor + ".framenum").each(function () { this.innerHTML = framenum; });
            if (words[3].startsWith("data:image/png")) {
                $("." + sensor + ".latest").each(function () { this.src = payload; });
            } else {
                $("." + sensor + ".latest").each(function() {
                    $(this).width($(this).parent().width());
                    $(this).height($(this).parent().width() * 0.75);

                    var data = JSON.parse(payload);

                    // unround
                    for (var k in data) {
                        data[k] = data[k].map(x => x / 1000);
                    }

                    // merge with the data we have
                    if (sensor in SENSOR_DATA) {
                        overlap = SENSOR_DATA[sensor].t.indexOf(data.t[0]);
                        console.log(overlap);
                        if (overlap != -1) {
                            for (var k in SENSOR_DATA[sensor]) {
                                SENSOR_DATA[sensor][k]  = SENSOR_DATA[sensor][k].slice(0, overlap).concat(data[k]);
                            }
                        } else {
                            for (var k in SENSOR_DATA[sensor]) {
                                SENSOR_DATA[sensor][k]  = SENSOR_DATA[sensor][k].concat(data[k]);
                            }
                        }

                        if (SENSOR_DATA[sensor].t[SENSOR_DATA[sensor].t.length-1] - SENSOR_DATA[sensor].t[0] > 10) {
                            var start_time = SENSOR_DATA[sensor].t[SENSOR_DATA[sensor].t.length-1] - 10;
                            var start_idx = SENSOR_DATA[sensor].t.findIndex(function(t) { return t > start_time; });
                            for (var k in SENSOR_DATA[sensor]) {
                                SENSOR_DATA[sensor][k]  = SENSOR_DATA[sensor][k].slice(start_idx);
                            }
                        }
                    } else {
                        SENSOR_DATA[sensor] = data;
                    }

                    var lines = [];
                    var bbox = [
                        /* left */   SENSOR_DATA[sensor].t[0],
                        /* top  */   -Infinity,
                        /* right */  SENSOR_DATA[sensor].t[SENSOR_DATA[sensor].t.length-1],
                        /* bottom */ Infinity
                    ];
                    for (var k in SENSOR_DATA[sensor]) {
                        if (k == 't') continue;
                        lines.push({ name: k, data: [SENSOR_DATA[sensor].t, SENSOR_DATA[sensor][k]] });
                        bbox[0] = Math.min(bbox[0], SENSOR_DATA[sensor].t[0]);
                        bbox[2] = Math.max(bbox[2], SENSOR_DATA[sensor].t[SENSOR_DATA[sensor].t.length-1]);
                        bbox[1] = Math.max(bbox[1], SENSOR_DATA[sensor][k].reduce((a, b) => a > b ? a : b));
                        bbox[3] = Math.min(bbox[1], SENSOR_DATA[sensor][k].reduce((a, b) => a < b ? a : b));
                    }
                    var tic = new Date();
                    if (typeof this.board !== 'undefined') {
                        JXG.JSXGraph.freeBoard(this.board);
                    }
                    this.board = JXG.JSXGraph.initBoard('chart-container-' + sensor, {
                        boundingbox: bbox,
                        axis: true
                    });
                    this.board.suspendUpdate();
                    var colors = ['red', 'green', 'blue', 'black'];
                    for (var l in lines) {
                        this.board.create('curve', lines[l].data, {
                            name: lines[l].name,
                            strokeColor: colors[l]
                        });
                    }
                    this.board.create('legend',
                            [bbox[0] + (bbox[2]-bbox[0])*.75,
                             bbox[3] + (bbox[1]-bbox[3])*.5],
                             {
                                 labels: lines.map(l => l.name),
                                 colors: colors,
                                 linelength: (bbox[2]-bbox[0])*.1
                             });
                    this.board.unsuspendUpdate();
                    var toc = new Date();
                    console.log(toc - tic);
                    if (DEMO && sensor in DRAW_TIMINGS) {
                        DRAW_TIMINGS[sensor].push({'num': framenum, 'time': toc - tic});
                    }
                });
            }
            if (DEMO && sensor in FRAME_TIMINGS) {
                FRAME_TIMINGS[sensor].push({'num': framenum, 'time': new Date()});
            }
            break;
        case "panic":
            serv = words[1];
            $("#light-" + serv).css("background-color", "blue");
            alert("The " + serv + " thread crashed! (" + words.slice(2).join(" ") + ")\n\nIf it was running, you may want to click Start again.");
            break;
        case "flow":
            $("#flows").html(event.data.slice(event.data.indexOf(" ")));
            break;
        case "start":
            serv = words[1];
            $("#light-" + serv).css("background-color", "green");
            break;
        case "stop":
            serv = words[1];
            $("#light-" + serv).css("background-color", "red");
            break;
        case "diskfree":
            $("#datadir").html(words[1]);
            $("#diskfree").html(words[2]);
            break;
        case "location":
            $.ajax({
                url: 'https://www.googleapis.com/geolocation/v1/geolocate?key=AIzaSyBLp5ElrEuwr3N9_Xxq7RnQV4E4vjzybS8',
                method: 'POST'
            }).done(function (data) {
                send("GPS" + window.wsid + " " + data.location.lat + " " + data.location.lng);
            });
            break;
    }
};
window.socket.onopen = function (event) {
    console.log("Server connection ready!");
};
window.socket.onclose = function (event) {
    console.log("Server connection lost!");
};
window.socket.onerror = function (event) {
    console.log("Server connection error!");
};

