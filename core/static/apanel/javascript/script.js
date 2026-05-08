$(function(){

    $('a.window-box, .nav > .all-comments').colorbox({
                                              width         : '92%',
                                              height        : '90%',
                                              maxHeight     : 800,
                                              maxWidth      : 1200,
                                              initialWidth  : 800,
                                              initialHeight : 600,
                                              onComplete: function () {
                                                   var h = $('#cboxLoadedContent').height();
                                                   $('#fb-work-comm').css({'height' : (h - 142) + 'px'});
                                              }
    });

    $('textarea:not(.noresize)').TextAreaResizer();
    
    $('.panels').click(function() {
        var id = $(this).attr('id');
        if($(this).hasClass('menupanelopen')){
            $(this).next().slideUp();
            $(this).removeClass('menupanelopen');
            $.cookie('openmenu','');
        } else {
            if(id){ $.cookie('openmenu',id); }
            $('.panels').removeClass('menupanelopen');
            $('.mcont').slideUp();
            $(this).addClass('menupanelopen');
            $(this).next().slideDown();
        }
        return false;
    }); 
                  
    $('#acc').bind('change', function() {
        if ($(this).val() == 'group') {
            $('#group').slideDown();
        } else {
            $('#group').slideUp();
        }
    }); 
  
}); 

function globalnotice(title, description, url_image, class_name) {
    var unique_id = $.gritter.add({
                      title  : title,
                      text   : description,
                      image  : url_image,
                      sticky : true, 
                      time   : '', 
                      iclass : class_name 
    });
}
              
(function($){
     $.fn.reload = function(){
	$(this).click(function(){
		reload(this);
		return true;
	});
	function reload() {
	    $('body').append('<div id="overlay" />');
	    $('body').css({height:'100%'});
	    $('#overlay').css({
				display: 'none',
				position: 'absolute',
				top:0,
				left: 0,
				width: '100%',
				height: '100%',
				zIndex: 1000,
				background: 'black',
				opacity : 1
			       })
                               .fadeIn(400);  
	    $('#overlay').fadeOut(400);
	}
    };
})(jQuery);

(function($){
     $.fn.tooltip = function(){
          var delay = 300, show, o = {xd : 15, yd : 19, top : 0, left : 0, x : 0, y : 0, mx : 0, my : 0, mc : 0};
          $('body').append('<div id="tooltip" style="display:none;position:absolute;"></div>');
          this.each(function(){
                var h = ($(this).is("img")) ? $(this).attr('alt') : $(this).attr('title');
                if (!$(this).is(".notooltip")){
                    $(this).bind('mouseover mousemove', function(e){
                        if (h) {
                            $('#tooltip').html(h);
                            ($(this).is("img")) ? $(this).attr('alt','') : $(this).attr('title','');
                            if ($('#tooltip').width() > 200) {
                        	$('#tooltip').css({'width':'200px'});
                            }
                            o.mc = document.getElementsByTagName((document.compatMode && document.compatMode == "CSS1Compat") ? "HTML":"BODY")[0];
                            o.left = e.pageX + o.xd;
                            o.top = e.pageY + o.yd;
                            o.mx = window.event ? event.clientX + o.mc.scrollLeft : e.pageX;
                            o.my = window.event ? event.clientY + o.mc.scrollTop : e.pageY;
                            if ((o.mx + $('#tooltip').width() + o.xd)  > (o.mc.clientWidth ? o.mc.clientWidth + o.mc.scrollLeft : window.innerWidth + window.pageXOffset) - 35) {
                                o.left = (o.mx - $('#tooltip').width() - 15);
                            }
                            if ((o.my + $('#tooltip').height() + o.yd) > (o.mc.innerHeight ? window.innerHeight + window.pageYOffset :o.mc.clientHeight + o.mc.scrollTop) - 50) {
                                o.top = (o.my - $('#tooltip').height() - 20);
                            }
                            if (e.type == 'mouseover') {
                                show = window.setTimeout(function() {
                                    $('#tooltip').fadeIn(300).show().css({'top':o.top+'px','left':o.left+'px'});
                                }, delay);
                            }
                            if (e.type == 'mousemove') {
                                $('#tooltip').css({'top':o.top+'px','left':o.left+'px'});
                            }
                        }
                   });
                   $(this).mouseout(function(e){
                        $('#tooltip').hide();
                        $('#tooltip').css({'width':''});
                        window.clearTimeout(show);
                        ($(this).is("img")) ? $(this).attr('alt',h) : $(this).attr('title',h);
                   }); 
               }
          });
     };
})(jQuery);

/*
	jQuery TextAreaResizer plugin
	Created on 17th January 2008 by Ryan O'Dell
	Version 1.0.4

	Converted from Drupal -> textarea.js
	Found source: http://plugins.jquery.com/misc/textarea.js
	$Id: textarea.js,v 1.11.2.1 2007/04/18 02:41:19 drumm Exp $

	1.0.1 Updates to missing global 'var', added extra global variables, fixed multiple instances, improved iFrame support
	1.0.2 Updates according to textarea.focus
	1.0.3 Further updates including removing the textarea.focus and moving private variables to top
	1.0.4 Re-instated the blur/focus events, according to information supplied by dec


*/
(function($) {
	/* private variable "oHover" used to determine if you're still hovering over the same element */
	var textarea, staticOffset;  // added the var declaration for 'staticOffset' thanks to issue logged by dec.
	var iLastMousePos = 0;
	var iMin = 32;
	var grip;
	/* TextAreaResizer plugin */
	$.fn.TextAreaResizer = function() {
		return this.each(function() {
		    //textarea = $(this).addClass('processed'), staticOffset = null;
            textarea = $(this), staticOffset = null;
			// 18-01-08 jQuery bind to pass data element rather than direct mousedown - Ryan O'Dell
		    // When wrapping the text area, work around an IE margin bug.  See:
		    // http://jaspan.com/ie-inherited-margin-bug-form-elements-and-haslayout
		    $(this).wrap('<div class="resizable-textarea"><span></span></div>')
		      .parent().append($('<div class="grippie"></div>').bind("mousedown",{el: this} , startDrag));

		    var grippie = $('div.grippie', $(this).parent())[0];
		    grippie.style.marginRight = (grippie.offsetWidth - $(this)[0].offsetWidth) +'px';

		});
	};
	/* private functions */
	function startDrag(e) {
		textarea = $(e.data.el);
		//textarea.blur();
		iLastMousePos = mousePosition(e).y;
		staticOffset = textarea.height() - iLastMousePos;
		textarea.css({'opacity': 0.7, 'border': '1px solid #d30'});
		$(document).mousemove(performDrag).mouseup(endDrag);
		return false;
	}

	function performDrag(e) {
		var iThisMousePos = mousePosition(e).y;
		var iMousePos = staticOffset + iThisMousePos;
		if (iLastMousePos >= (iThisMousePos)) {
			iMousePos -= 5;
		}
		iLastMousePos = iThisMousePos;
		iMousePos = Math.max(iMin, iMousePos);
		textarea.height(iMousePos + 'px');
		if (iMousePos < iMin) {
			endDrag(e);
		}
		return false;
	}

	function endDrag(e) {
		$(document).unbind('mousemove', performDrag).unbind('mouseup', endDrag);
		textarea.css({'opacity': 1, 'border': '1px solid #9ab'});
		//textarea.focus();
		textarea = null;
		staticOffset = null;
		iLastMousePos = 0;
	}

	function mousePosition(e) {
		return { x: e.clientX + document.documentElement.scrollLeft, y: e.clientY + document.documentElement.scrollTop };
	};
})(jQuery);

/*
 * Gritter for jQuery
 * http://www.boedesign.com/
 *
 * Copyright (c) 2009 Jordan Boesch
 * Dual licensed under the MIT and GPL licenses.
 *
 * Date: June 26, 2009
 * Version: 1.0
 */

jQuery(document).ready(function($){
 	
 	/********************************************
	 * First, we'll define our object
	 */

	Gritter = {

	    // PUBLIC - touch all you want
		fade_speed: 2000, // how fast the notices fade out
	    timer_stay: 6000, // how long you want the message to hang on screen for
	    
	    // PRIVATE - no touchy the private parts
		_custom_timer: 0,
	    _item_count: 0,
		_tpl_close: '<div class="gritter-close"></div>',
		_tpl_item: '<div id="gritter-item-[[number]]" class="gritter-item-wrapper" style="display:none"><div class="[[class_name]] gritter-item">[[image]]<div class="gritter-with-image"><span class="gritter-title">[[username]]</span><p>[[text]]</p></div><div style="clear:both"></div></div></div>',
	    _tpl_wrap: '<div id="gritter-notice-wrapper"></div>',
	    
	    // Add a notification to the screen
	    add: function(user, text, image, sticky, time_alive, class_name){
	        
	        // This is also called from init, we just added it here because
	        // some people might just call the "add" method
	        this.verifyWrapper();
	        
	        var tmp = this._tpl_item;
	        this._item_count++;
			
			// reset
			this._custom_timer = 0;
			
			// a custom fade time set
			if(time_alive){
				this._custom_timer = time_alive;
			}
			
			var image_str = (image != '') ? '<img src="' + image + '" class="gritter-image" />' : '';
			var class_name = (class_name == '') ? '' :  class_name;
			
	        tmp = this.str_replace(
	            ['[[username]]', '[[text]]', '[[image]]', '[[number]]', '[[time_alive]]', '[[class_name]]'],
	            [user, text, image_str, this._item_count, time_alive, class_name], tmp
	        );
	        
	        $('#gritter-notice-wrapper').append(tmp);
	        var item = $('#gritter-item-' + this._item_count);
	        var number = this._item_count;
	        item.fadeIn();
	        
			if(!sticky){
				this.setFadeTimer(item, number);
			}
			
			$(item).hover(function(){
				if(!sticky){ 
					Gritter.restoreItemIfFading(this, number);
				}
				Gritter.hoveringItem(this);
			},
			function(){
				if(!sticky){
					Gritter.setFadeTimer(this, number);
				}
				Gritter.unhoveringItem(this);
			});
			
			return number;
	    
	    },
		
		// If we don't have any more gritter notifications, get rid of the wrapper
	    countRemoveWrapper: function(){
	        
	        if($('.gritter-item-wrapper').length == 0){
	            $('#gritter-notice-wrapper').remove();
	        }
	    
	    },
		
		// Fade the item and slide it up nicely... once its completely faded, remove it
	    fade: function(e){
	
	        $(e).animate({
	            opacity:0
	        }, Gritter.fade_speed, function(){
	            $(e).animate({ height: 0 }, 300, function(){
	                $(e).remove();
	                Gritter.countRemoveWrapper();
	            })
	        })
	        
	    },
		
		 // Change the border styles and add the (X) close button when you hover
	    hoveringItem: function(e){
	    	
	    	$(e).addClass('hover');
	    	
			if($(e).find('img').length){
				$(e).find('img').before(this._tpl_close);
			}
			else {
				$(e).find('span').before(this._tpl_close);
			}
			$(e).find('.gritter-close').click(function(){
				Gritter.remove(this);
			});
	        
	    },
	    
	    // Remove a notification, this is called from the inline "onclick" event
	    remove: function(e){
	        
	        $(e).parents('.gritter-item-wrapper').fadeOut('medium', function(){ $(this).remove();  });
	        this.countRemoveWrapper();
	        
			$.cookie('alerts', 'off', { expires: 2 }); // set cookie with an expiration date seven days in the future
	    },
		
		// Remove a specific notification based on an id (int)
		removeSpecific: function(id, params){
			
			var e = $('#gritter-item-' + id);
			if(typeof(params) === 'object'){
				if(params.fade){
					var speed = this.fade_speed;
					if(params.speed){
						speed = params.speed;
					}
					e.fadeOut(speed);
				}
			}
			else {
				e.remove();
			}
			
			this.countRemoveWrapper();
	
		},
		
		 // If the item is fading out and we hover over it, restore it!
	    restoreItemIfFading: function(e, number){
			
			window.clearTimeout(Gritter['_int_id_' + number]);
	        $(e).stop().css({ opacity: 1 });
	        
	    },
	    
	    // Set the notification to fade out after a certain amount of time
	    setFadeTimer: function(item, number){
			
			var timer_str = (this._custom_timer) ? this._custom_timer : this.timer_stay;
	        Gritter['_int_id_' + number] = window.setTimeout(function(){ Gritter.fade(item); }, timer_str);
	
	    },
		
		// Bring everything to a halt!    
		stop: function(){
	
			$('#gritter-notice-wrapper').fadeOut(function(){
				$(this).remove();
			});
	
		},
		
		// A handy PHP function ported to js!
	    str_replace: function(search, replace, subject, count) {
	    
	        var i = 0, j = 0, temp = '', repl = '', sl = 0, fl = 0,
	            f = [].concat(search),
	            r = [].concat(replace),
	            s = subject,
	            ra = r instanceof Array, sa = s instanceof Array;
	        s = [].concat(s);
	        if (count) {
	            this.window[count] = 0;
	            }
	 
	        for (i=0, sl=s.length; i < sl; i++) {
	            if (s[i] === '') {
	                continue;
	            }
	            for (j=0, fl=f.length; j < fl; j++) {
	                temp = s[i]+'';
	                repl = ra ? (r[j] !== undefined ? r[j] : '') : r[0];
	                s[i] = (temp).split(f[j]).join(repl);
	                if (count && s[i] !== temp) {
	                    this.window[count] += (temp.length-s[i].length)/f[j].length;}
	            }
	        }
	        return sa ? s : s[0];
	        
	    },
	    
	    // Remove the border styles and (X) close button when you mouse out
	    unhoveringItem: function(e){
	        
	        $(e).removeClass('hover');
	        $(e).find('.gritter-close').remove();
	        
	    },
		
		// Make sure we have something to wrap our notices with
		verifyWrapper: function(){
	      
			if($('#gritter-notice-wrapper').length == 0){
				$('body').append(this._tpl_wrap);
			}
	 
		}
	    
	}
	
	/********************************************
	 * Now lets turn it into some jQuery Magic!
	 */
	
	// Set it up as an object
	$.gritter = {};
	
	// Add a gritter notification
	$.gritter.add = function(params){

		try {
			if(!params.title || !params.text){
				throw "Missing_Fields"; 
			}
		} catch(e) {
			if(e == "Missing_Fields"){
				alert('Gritter Error: You need to fill out the first 2 params: "title" and "text"');
			}
		}
		
		var id = Gritter.add(
			params.title,
			params.text,
			params.image || '',
			params.sticky || false,
			params.time || '',
			params.iclass || ''
		);
		
		return id;

	}
	
	// Remove a specific notification
	$.gritter.remove = function(id, params){
		Gritter.removeSpecific(id, params || '');
	}
	
	// Remove all gritter notifications
	$.gritter.removeAll = function(){
		Gritter.stop();
	}
	
});
